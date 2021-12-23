use {
  borsh::{BorshDeserialize, BorshSerialize},
};

// Constants for working with pyth's number representation
const PD_EXPO: i32 = -9;
const PD_SCALE: u64 = 1_000_000_000;
const MAX_PD_V_U64: u64 = (1 << 28) - 1;
const MAX_PD_V_I64: i64 = MAX_PD_V_U64 as i64;
const MIN_PD_V_I64: i64 = -MAX_PD_V_I64;

/**
 * A price with a degree of uncertainty, represented as a price +- a confidence interval.
 * The confidence interval roughly corresponds to the standard error of a normal distribution.
 * Both the price and confidence are stored in a fixed-point numeric representation, `x * 10^expo`,
 * where `expo` is the exponent. For example:
 *
 * ```
 * use pyth_client::PriceConf;
 * PriceConf { price: 12345, conf: 267, expo: -2 }; // represents 123.45 +- 2.67
 * PriceConf { price: 123, conf: 1, expo: 2 }; // represents 12300 +- 100
 * ```
 */
#[derive(PartialEq, Debug, BorshSerialize, BorshDeserialize, Clone)]
pub struct PriceConf {
  pub price: i64,
  pub conf: u64,
  pub expo: i32,
}

impl PriceConf {
  /**
   * Divide this price by `other` while propagating the uncertainty in both prices into the result.
   * The uncertainty propagation algorithm is an approximation due to computational limitations
   * that may slightly overestimate the resulting uncertainty (by at most a factor of sqrt(2)).
   *
   * This method will automatically select a reasonable exponent for the result. If both
   * `self` and `other` are normalized, the exponent is `self.expo + PD_EXPO - other.expo` (i.e.,
   * the fraction has `PD_EXPO` digits of additional precision). If they are not normalized,
   * this method will normalize them, resulting in an unpredictable result exponent.
   * If the result is used in a context that requires a specific exponent, please call
   * `scale_to_exponent` on it.
   *
   * This function will return `None` unless all of the following conditions are satisfied:
   * 1. The prices of self and other are > 0.
   * 2. The confidence of the result can be represented using a 64-bit number in the computed
   *    exponent. This condition will fail if the confidence is >> the price of either input,
   *    (which should almost never occur in the real world)
   */
  pub fn div(&self, other: &PriceConf) -> Option<PriceConf> {
    // PriceConf is not guaranteed to store its price/confidence in normalized form.
    // Normalize them here to bound the range of price/conf, which is required to perform
    // arithmetic operations.
    let base = self.normalize()?;
    let other = other.normalize()?;

    if other.price == 0 {
      return None;
    }

    // These use at most 27 bits each
    let (base_price, base_sign) = PriceConf::to_unsigned(base.price);
    let (other_price, other_sign) = PriceConf::to_unsigned(other.price);

    // Compute the midprice, base in terms of other.
    // Uses at most 57 bits
    let midprice = base_price * PD_SCALE / other_price;
    let midprice_expo = base.expo.checked_sub(other.expo)?.checked_add(PD_EXPO)?;

    // Compute the confidence interval.
    // This code uses the 1-norm instead of the 2-norm for computational reasons.
    // The correct formula is midprice * sqrt(c_1^2 + c_2^2), where c_1 and c_2 are the
    // confidence intervals in price-percentage terms of the base and other. This quantity
    // is difficult to compute due to the sqrt, and overflow/underflow considerations.
    // Instead, this code uses midprice * (c_1 + c_2).
    // This quantity is at most a factor of sqrt(2) greater than the correct result, which
    // shouldn't matter considering that confidence intervals are typically ~0.1% of the price.

    // The exponent is PD_EXPO for both of these. Each of these uses 57 bits.
    // let base_confidence_pct: u64 = (base.conf * PD_SCALE) / base_price;
    let other_confidence_pct: u64 = (other.conf * PD_SCALE) / other_price;

    // at most 58 bits
    // let confidence_pct = base_confidence_pct + other_confidence_pct;
    // at most 57 + 58 - 29 = 86 bits, with the same exponent as the midprice.
    // FIXME: round this up. There's a div_ceil method but it's unstable (?)
    // let conf = ((confidence_pct as u128) * (midprice as u128)) / (PD_SCALE as u128);

    let conf = (((base.conf * PD_SCALE) / other_price) as u128) + ((other_confidence_pct as u128) * (midprice as u128)) / (PD_SCALE as u128);

    // Note that this check only fails if an argument's confidence interval was >> its price,
    // in which case None is a reasonable result, as we have essentially 0 information about the price.
    if conf < (u64::MAX as u128) {
      Some(PriceConf {
        price: (midprice as i64) * base_sign * other_sign,
        conf: conf as u64,
        expo: midprice_expo,
      })
    } else {
      None
    }
  }

  // FIXME Implement these functions
  // The idea is that you should be able to get the price of a mixture of tokens (e.g., for LP tokens)
  // using something like:
  // price1.scale_to_exponent(result_expo).cmul(qty1, 0).add(
  //   price2.scale_to_exponent(result_expo).cmul(qty2, 0)
  // )
  //
  // Add two PriceConfs assuming the expos are ==
  pub fn add(&self, other: PriceConf) -> Option<PriceConf> {
    panic!()
  }

  // multiply by a constant
  pub fn cmul(&self, c: i64, e: i32) -> Option<PriceConf> {
    self.mul(&PriceConf { price: c, conf: 0, expo: e })
  }

  pub fn mul(&self, other: &PriceConf) -> Option<PriceConf> {
    // PriceConf is not guaranteed to store its price/confidence in normalized form.
    // Normalize them here to bound the range of price/conf, which is required to perform
    // arithmetic operations.
    let base = self.normalize()?;
    let other = other.normalize()?;

    // These use at most 27 bits each
    let (base_price, base_sign) = PriceConf::to_unsigned(base.price);
    let (other_price, other_sign) = PriceConf::to_unsigned(other.price);

    // Compute the midprice, base in terms of other.
    // Uses at most 27*2 bits
    let midprice = base_price * other_price;
    let midprice_expo = base.expo.checked_add(other.expo)?;

    // Compute the confidence interval.
    // This code uses the 1-norm instead of the 2-norm for computational reasons.
    // The correct formula is midprice * sqrt(c_1^2 + c_2^2), where c_1 and c_2 are the
    // confidence intervals in price-percentage terms of the base and other. This quantity
    // is difficult to compute due to the sqrt, and overflow/underflow considerations.
    // Instead, this code uses midprice * (c_1 + c_2).
    // This quantity is at most a factor of sqrt(2) greater than the correct result, which
    // shouldn't matter considering that confidence intervals are typically ~0.1% of the price.
    //
    // Note that this simplifies to
    // pq * (a/p + b/q) = qa + bp
    // 27*2 + 1 bits
    let conf = base.conf * other_price + other.conf * base_price;

    Some(PriceConf {
      price: (midprice as i64) * base_sign * other_sign,
      conf,
      expo: midprice_expo,
    })
  }

  /**
   * Get a copy of this struct where the price and confidence
   * have been normalized to be between MIN_PD_V_I64 and MAX_PD_V_I64.
   */
  pub fn normalize(&self) -> Option<PriceConf> {
    let mut p = self.price;
    let mut c = self.conf;
    let mut e = self.expo;

    while p > MAX_PD_V_I64 || p < MIN_PD_V_I64 || c > MAX_PD_V_U64 {
      p = p / 10;
      c = c / 10;
      e = e.checked_add(1)?;
    }

    Some(PriceConf {
      price: p,
      conf: c,
      expo: e,
    })
  }

  /**
   * Scale num so that its exponent is target_expo.
   * FIXME: tests
   */
  pub fn scale_to_exponent(
    &self,
    target_expo: i32,
  ) -> Option<PriceConf> {
    let mut delta = target_expo - self.expo;
    if delta >= 0 {
      let mut p = self.price;
      let mut c = self.conf;
      while delta > 0 {
        p /= 10;
        c /= 10;
        delta -= 1;
      }
      // FIXME: check for 0s here and handle this case more gracefully. (0, 0) is a bad answer that will cause bugs
      Some(PriceConf {
        price: p,
        conf: c,
        expo: target_expo,
      })
    } else {
      let mut p = Some(self.price);
      let mut c = Some(self.conf);

      while delta < 0 {
        p = p?.checked_mul(10);
        c = c?.checked_mul(10);
        delta += 1;
      }

      match (p, c) {
        (Some(price), Some(conf)) =>
          Some(PriceConf {
            price,
            conf,
            expo: target_expo,
          }),
        (_, _) => None,
      }
    }
  }

  fn to_unsigned(x: i64) -> (u64, i64) {
    assert!(x <= MAX_PD_V_I64 && x >= MIN_PD_V_I64);
    if (x < 0) {
      (-x as u64, -1)
    } else {
      (x as u64, 1)
    }
  }
}

#[cfg(test)]
mod test {
  use crate::price_conf::{MAX_PD_V_U64, MAX_PD_V_I64, MIN_PD_V_I64, PD_EXPO, PD_SCALE, PriceConf};

  fn pc(price: i64, conf: u64, expo: i32) -> PriceConf {
    PriceConf {
      price: price,
      conf: conf,
      expo: expo,
    }
  }

  fn pc_scaled(price: i64, conf: u64, cur_expo: i32, expo: i32) -> PriceConf {
    PriceConf {
      price: price,
      conf: conf,
      expo: cur_expo,
    }.scale_to_exponent(expo).unwrap()
  }

  #[test]
  fn test_normalize() {
    fn succeeds(
      price1: PriceConf,
      expected: PriceConf,
    ) {
      assert_eq!(price1.normalize().unwrap(), expected);
    }

    fn fails(
      price1: PriceConf,
    ) {
      assert_eq!(price1.normalize(), None);
    }

    succeeds(
      pc(2 * (PD_SCALE as i64), 3 * PD_SCALE, 0),
      pc(2 * (PD_SCALE as i64) / 100, 3 * PD_SCALE / 100, 2)
    );

    succeeds(
      pc(-2 * (PD_SCALE as i64), 3 * PD_SCALE, 0),
      pc(-2 * (PD_SCALE as i64) / 100, 3 * PD_SCALE / 100, 2)
    );

    // the max values are a factor of 10^11 larger than MAX_PD_V
    let expo = -(PD_EXPO - 2);
    let scale_i64 = (PD_SCALE as i64) * 100;
    let scale_u64 = scale_i64 as u64;
    succeeds(pc(i64::MAX, 1, 0), pc(i64::MAX / scale_i64, 0, expo));
    succeeds(pc(i64::MIN, 1, 0), pc(i64::MIN / scale_i64, 0, expo));
    succeeds(pc(1, u64::MAX, 0), pc(0, u64::MAX / scale_u64, expo));

    // exponent overflows
    succeeds(pc(i64::MAX, 1, i32::MAX - expo), pc(i64::MAX / scale_i64, 0, i32::MAX));
    fails(pc(i64::MAX, 1, i32::MAX - expo + 1));
    succeeds(pc(i64::MAX, 1, i32::MIN), pc(i64::MAX / scale_i64, 0, i32::MIN + expo));

    succeeds(pc(1, u64::MAX, i32::MAX - expo), pc(0, u64::MAX / scale_u64, i32::MAX));
    fails(pc(1, u64::MAX, i32::MAX - expo + 1));
  }

  #[test]
  fn test_div() {
    fn succeeds(
      price1: PriceConf,
      price2: PriceConf,
      expected: PriceConf,
    ) {
      assert_eq!(price1.div(&price2).unwrap(), expected);
    }

    fn fails(
      price1: PriceConf,
      price2: PriceConf,
    ) {
      let result = price1.div(&price2);
      assert_eq!(result, None);
    }

    succeeds(pc(1, 1, 0), pc(1, 1, 0), pc_scaled(1, 2, 0, PD_EXPO));
    succeeds(pc(1, 1, -8), pc(1, 1, -8), pc_scaled(1, 2, 0, PD_EXPO));
    succeeds(pc(10, 1, 0), pc(1, 1, 0), pc_scaled(10, 11, 0, PD_EXPO));
    succeeds(pc(1, 1, 1), pc(1, 1, 0), pc_scaled(10, 20, 0, PD_EXPO + 1));
    succeeds(pc(1, 1, 0), pc(5, 1, 0), pc_scaled(20, 24, -2, PD_EXPO));

    // Negative numbers
    succeeds(pc(-1, 1, 0), pc(1, 1, 0), pc_scaled(-1, 2, 0, PD_EXPO));
    succeeds(pc(1, 1, 0), pc(-1, 1, 0), pc_scaled(-1, 2, 0, PD_EXPO));
    succeeds(pc(-1, 1, 0), pc(-1, 1, 0), pc_scaled(1, 2, 0, PD_EXPO));

    // Different exponents in the two inputs
    succeeds(pc(100, 10, -8), pc(2, 1, -7), pc_scaled(500_000_000, 300_000_000, -8, PD_EXPO - 1));
    succeeds(pc(100, 10, -4), pc(2, 1, 0), pc_scaled(500_000, 300_000, -8, PD_EXPO + -4));

    // Test with end range of possible inputs where the output should not lose precision.
    succeeds(pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc_scaled(1, 2, 0, PD_EXPO));
    succeeds(pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc(1, 1, 0), pc_scaled(MAX_PD_V_I64, 2 * MAX_PD_V_U64, 0, PD_EXPO));
    succeeds(pc(1, 1, 0),
             pc(MAX_PD_V_I64, MAX_PD_V_U64, 0),
             pc((PD_SCALE as i64) / MAX_PD_V_I64, 2 * (PD_SCALE / MAX_PD_V_U64), PD_EXPO));

    succeeds(pc(MIN_PD_V_I64, MAX_PD_V_U64, 0), pc(MIN_PD_V_I64, MAX_PD_V_U64, 0), pc_scaled(1, 2, 0, PD_EXPO));
    succeeds(pc(MIN_PD_V_I64, MAX_PD_V_U64, 0), pc(1, 1, 0), pc_scaled(MIN_PD_V_I64, 2 * MAX_PD_V_U64, 0, PD_EXPO));
    succeeds(pc(1, 1, 0),
             pc(MIN_PD_V_I64, MAX_PD_V_U64, 0),
             pc((PD_SCALE as i64) / MIN_PD_V_I64, 2 * (PD_SCALE / MAX_PD_V_U64), PD_EXPO));

    succeeds(pc(1, MAX_PD_V_U64, 0), pc(1, MAX_PD_V_U64, 0), pc_scaled(1, 2 * MAX_PD_V_U64, 0, PD_EXPO));
    // This fails because the confidence interval is too large to be represented in PD_EXPO
    fails(pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc(1, MAX_PD_V_U64, 0));

    // Unnormalized tests below here

    // More realistic inputs (get BTC price in ETH)
    let ten_e7: i64 = 10000000;
    let uten_e7: u64 = 10000000;
    succeeds(pc(520010 * ten_e7, 310 * uten_e7, -8),
             pc(38591 * ten_e7, 18 * uten_e7, -8),
             pc(1347490347, 1431804, -8));

    // Test with end range of possible inputs to identify overflow
    // These inputs will lose precision due to the initial normalization.
    // Get the rounded versions of these inputs in order to compute the expected results.
    let normed = pc(i64::MAX, u64::MAX, 0).normalize().unwrap();

    succeeds(pc(i64::MAX, u64::MAX, 0), pc(i64::MAX, u64::MAX, 0), pc_scaled(1, 4, 0, PD_EXPO));
    succeeds(pc(i64::MAX, u64::MAX, 0),
             pc(1, 1, 0),
             pc_scaled(normed.price, 3 * (normed.price as u64), normed.expo, normed.expo + PD_EXPO));
    succeeds(pc(1, 1, 0),
             pc(i64::MAX, u64::MAX, 0),
             pc((PD_SCALE as i64) / normed.price, 3 * (PD_SCALE / (normed.price as u64)), PD_EXPO - normed.expo));

    succeeds(pc(i64::MAX, 1, 0), pc(i64::MAX, 1, 0), pc_scaled(1, 0, 0, PD_EXPO));
    succeeds(pc(i64::MAX, 1, 0),
             pc(1, 1, 0),
             pc_scaled(normed.price, normed.price as u64, normed.expo, normed.expo + PD_EXPO));
    succeeds(pc(1, 1, 0),
             pc(i64::MAX, 1, 0),
             pc((PD_SCALE as i64) / normed.price, PD_SCALE / (normed.price as u64), PD_EXPO - normed.expo));

    // TODO: negative number tests around i64::MIN

    // Price is zero pre-normalization
    succeeds(pc(0, 1, 0), pc(1, 1, 0), pc_scaled(0, 1, 0, PD_EXPO));
    succeeds(pc(0, 1, 0), pc(100, 1, 0), pc_scaled(0, 1, -2, PD_EXPO));
    fails(pc(1, 1, 0), pc(0, 1, 0));

    // Normalizing the input when the confidence is >> price produces a price of 0.
    fails(pc(1, 1, 0), pc(1, u64::MAX, 0));
    succeeds(
      pc(1, u64::MAX, 0),
      pc(1, 1, 0),
      pc_scaled(0, normed.conf, normed.expo, normed.expo + PD_EXPO)
    );

    // Exponent under/overflow.
    succeeds(pc(1, 1, i32::MAX), pc(1, 1, 0), pc(PD_SCALE as i64, 2 * PD_SCALE, i32::MAX + PD_EXPO));
    fails(pc(1, 1, i32::MAX), pc(1, 1, -1));

    succeeds(pc(1, 1, i32::MIN - PD_EXPO), pc(1, 1, 0), pc(PD_SCALE as i64, 2 * PD_SCALE, i32::MIN));
    succeeds(pc(1, 1, i32::MIN), pc(1, 1, PD_EXPO), pc(PD_SCALE as i64, 2 * PD_SCALE, i32::MIN));
    fails(pc(1, 1, i32::MIN - PD_EXPO), pc(1, 1, 1));

    // FIXME: move to scaling tests
    // Result exponent too small
    /*
    test_succeeds(pc(1, 1, 0), pc(1, 1, 0), PD_EXPO, (1 * (PD_SCALE as i64), 2 * PD_SCALE));
    test_fails(pc(1, 1, 0), pc(1, 1, 0), PD_EXPO - 1);
    */
  }

  #[test]
  fn test_mul() {
    fn succeeds(
      price1: PriceConf,
      price2: PriceConf,
      expected: PriceConf,
    ) {
      assert_eq!(price1.mul(&price2).unwrap(), expected);
    }

    fn fails(
      price1: PriceConf,
      price2: PriceConf,
    ) {
      let result = price1.mul(&price2);
      assert_eq!(result, None);
    }

    // TODO: test negative numbers

    succeeds(pc(1, 1, 0), pc(1, 1, 0), pc(1, 2, 0));
    succeeds(pc(1, 1, -8), pc(1, 1, -8), pc(1, 2, -16));
    succeeds(pc(10, 1, 0), pc(1, 1, 0), pc(10, 11, 0));
    succeeds(pc(1, 1, 1), pc(1, 1, 0), pc(1, 2, 1));
    succeeds(pc(1, 1, 0), pc(5, 1, 0), pc(5, 6, 0));

    // Different exponents in the two inputs
    succeeds(pc(100, 10, -8), pc(2, 1, -7), pc(200, 120, -15));
    succeeds(pc(100, 10, -4), pc(2, 1, 0), pc(200, 120, -4));

    // Zero
    succeeds(pc(0, 10, -4), pc(2, 1, 0), pc(0, 20, -4));
    succeeds(pc(2, 1, 0), pc(0, 10, -4), pc(0, 20, -4));

    // Test with end range of possible inputs where the output should not lose precision.
    succeeds(
      pc(MAX_PD_V_I64, MAX_PD_V_U64, 0),
      pc(MAX_PD_V_I64, MAX_PD_V_U64, 0),
      pc(MAX_PD_V_I64 * MAX_PD_V_I64, 2 * MAX_PD_V_U64 * MAX_PD_V_U64, 0)
    );
    succeeds(pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc(1, 1, 0), pc(MAX_PD_V_I64, 2 * MAX_PD_V_U64, 0));
    succeeds(
      pc(1, MAX_PD_V_U64, 0),
      pc(3, 1, 0),
      pc(3, 1 + 3 * MAX_PD_V_U64, 0)
    );

    succeeds(pc(1, MAX_PD_V_U64, 0), pc(1, MAX_PD_V_U64, 0), pc(1, 2 * MAX_PD_V_U64, 0));
    succeeds(
      pc(MAX_PD_V_I64, MAX_PD_V_U64, 0),
      pc(1, MAX_PD_V_U64, 0),
      pc(MAX_PD_V_I64, MAX_PD_V_U64 + MAX_PD_V_U64 * MAX_PD_V_U64, 0)
    );

    // Unnormalized tests below here
    let ten_e7: i64 = 10000000;
    let uten_e7: u64 = 10000000;
    succeeds(
      pc(3 * (PD_SCALE as i64), 3 * PD_SCALE, PD_EXPO),
      pc(2 * (PD_SCALE as i64), 4 * PD_SCALE, PD_EXPO),
      pc(6 * ten_e7 * ten_e7, 18 * uten_e7 * uten_e7, -14)
    );

    // Test with end range of possible inputs to identify overflow
    // These inputs will lose precision due to the initial normalization.
    // Get the rounded versions of these inputs in order to compute the expected results.
    let normed = pc(i64::MAX, u64::MAX, 0).normalize().unwrap();

    succeeds(
      pc(i64::MAX, u64::MAX, 0),
      pc(i64::MAX, u64::MAX, 0),
      pc(normed.price * normed.price, 4 * ((normed.price * normed.price) as u64), normed.expo * 2)
    );
    succeeds(pc(i64::MAX, u64::MAX, 0),
             pc(1, 1, 0),
             pc(normed.price, 3 * (normed.price as u64), normed.expo));

    succeeds(
      pc(i64::MAX, 1, 0),
      pc(i64::MAX, 1, 0),
      pc(normed.price * normed.price, 0, normed.expo * 2)
    );
    succeeds(pc(i64::MAX, 1, 0),
             pc(1, 1, 0),
             pc(normed.price, normed.price as u64, normed.expo));

    // Exponent under/overflow.
    succeeds(pc(1, 1, i32::MAX), pc(1, 1, 0), pc(1, 2, i32::MAX));
    succeeds(pc(1, 1, i32::MAX), pc(1, 1, -1), pc(1, 2, i32::MAX - 1));
    fails(pc(1, 1, i32::MAX), pc(1, 1, 1));

    succeeds(pc(1, 1, i32::MIN), pc(1, 1, 0), pc(1, 2, i32::MIN));
    succeeds(pc(1, 1, i32::MIN), pc(1, 1, 1), pc(1, 2, i32::MIN + 1));
    fails(pc(1, 1, i32::MIN), pc(1, 1, -1));


    /*
    // Price is zero pre-normalization
    fails(pc(0, 1, 0), pc(1, 1, 0));
    fails(pc(1, 1, 0), pc(0, 1, 0));

    // Can't normalize the input when the confidence is >> price.
    fails(pc(1, 1, 0), pc(1, u64::MAX, 0));
    fails(pc(1, u64::MAX, 0), pc(1, 1, 0));

    // FIXME: move to scaling tests
    // Result exponent too small
    test_succeeds(pc(1, 1, 0), pc(1, 1, 0), PD_EXPO, (1 * (PD_SCALE as i64), 2 * PD_SCALE));
    test_fails(pc(1, 1, 0), pc(1, 1, 0), PD_EXPO - 1);
    */
  }
}
