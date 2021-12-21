pub const MAGIC          : u32   = 0xa1b2c3d4;
pub const VERSION_2      : u32   = 2;
pub const VERSION        : u32   = VERSION_2;
pub const MAP_TABLE_SIZE : usize = 640;
pub const PROD_ACCT_SIZE : usize = 512;
pub const PROD_HDR_SIZE  : usize = 48;
pub const PROD_ATTR_SIZE : usize = PROD_ACCT_SIZE - PROD_HDR_SIZE;

// Constants for working with pyth's number representation
const PD_EXPO: i32 = -9;
const PD_SCALE: u64 = 1_000_000_000;
const MAX_PD_V_I64: i64 = (1 << 28) - 1;
const MAX_PD_V_U64: u64 = (1 << 28) - 1;

// each account has its own type
#[repr(C)]
pub enum AccountType
{
  Unknown,
  Mapping,
  Product,
  Price
}

// aggregate and contributing prices are associated with a status
// only Trading status is valid
#[repr(C)]
pub enum PriceStatus
{
  Unknown,
  Trading,
  Halted,
  Auction
}

// ongoing coporate action event - still undergoing dev
#[repr(C)]
pub enum CorpAction
{
  NoCorpAct
}

// different types of prices associated with a product
#[repr(C)]
pub enum PriceType
{
  Unknown,
  Price
}

// solana public key
#[repr(C)]
pub struct AccKey
{
  pub val: [u8;32]
}

// Mapping account structure
#[repr(C)]
pub struct Mapping
{
  pub magic      : u32,        // pyth magic number
  pub ver        : u32,        // program version
  pub atype      : u32,        // account type
  pub size       : u32,        // account used size
  pub num        : u32,        // number of product accounts
  pub unused     : u32,
  pub next       : AccKey,     // next mapping account (if any)
  pub products   : [AccKey;MAP_TABLE_SIZE]
}

// Product account structure
#[repr(C)]
pub struct Product
{
  pub magic      : u32,        // pyth magic number
  pub ver        : u32,        // program version
  pub atype      : u32,        // account type
  pub size       : u32,        // price account size
  pub px_acc     : AccKey,     // first price account in list
  pub attr       : [u8;PROD_ATTR_SIZE] // key/value pairs of reference attr.
}

// contributing or aggregate price component
#[repr(C)]
pub struct PriceInfo
{
  pub price      : i64,        // product price
  pub conf       : u64,        // confidence interval of product price
  pub status     : PriceStatus,// status of price (Trading is valid)
  pub corp_act   : CorpAction, // notification of any corporate action
  pub pub_slot   : u64
}

// latest component price and price used in aggregate snapshot
#[repr(C)]
pub struct PriceComp
{
  pub publisher  : AccKey,     // key of contributing quoter
  pub agg        : PriceInfo,  // contributing price to last aggregate
  pub latest     : PriceInfo   // latest contributing price (not in agg.)
}

#[repr(C)]
pub struct Ema
{
  pub val        : i64,        // current value of ema
  numer          : i64,        // numerator state for next update
  denom          : i64         // denominator state for next update
}

// Price account structure
#[repr(C)]
pub struct Price
{
  pub magic      : u32,        // pyth magic number
  pub ver        : u32,        // program version
  pub atype      : u32,        // account type
  pub size       : u32,        // price account size
  pub ptype      : PriceType,  // price or calculation type
  pub expo       : i32,        // price exponent
  pub num        : u32,        // number of component prices
  pub num_qt     : u32,        // number of quoters that make up aggregate
  pub last_slot  : u64,        // slot of last valid (not unknown) aggregate price
  pub valid_slot : u64,        // valid slot-time of agg. price
  pub twap       : Ema,        // time-weighted average price
  pub twac       : Ema,        // time-weighted average confidence interval
  pub drv1       : i64,        // space for future derived values
  pub drv2       : i64,        // space for future derived values
  pub prod       : AccKey,     // product account key
  pub next       : AccKey,     // next Price account in linked list
  pub prev_slot  : u64,        // valid slot of previous update
  pub prev_price : i64,        // aggregate price of previous update
  pub prev_conf  : u64,        // confidence interval of previous update
  pub drv3       : i64,        // space for future derived values
  pub agg        : PriceInfo,  // aggregate price info
  pub comp       : [PriceComp;32] // price components one per quoter
}

impl Price {
  /**
   * Get the current price and confidence interval as fixed-point numbers of the form a * 10^e.
   * Returns a struct containing the current price, confidence interval, and the exponent for both
   * numbers. Returns None if price information is currently unavailable.
   */
  pub fn get_current_price(&self) -> Option<PriceConf> {
    if !matches!(self.agg.status, PriceStatus::Trading) {
      None
    } else {
      Some(PriceConf {
        price: self.agg.price,
        conf: self.agg.conf,
        expo: self.expo
      })
    }
  }

  /**
   * Get the current price of this account in a different quote currency. If this account
   * represents the price of the product X/Z, and `quote` represents the price of the product Y/Z,
   * this method returns the price of X/Y. Use this method to get the price of e.g., mSOL/SOL from
   * the mSOL/USD and SOL/USD accounts.
   *
   * `result_expo` determines the exponent of the result, i.e., the number of digits of precision in
   * the price. For any given base/quote pair, the minimum possible exponent is
   * `-9 + self.exponent - quote.exponent`. (This minimum exponent reflects the maximum possible
   * precision for the result given the precision of the two inputs and the numeric representation.)
   */
  pub fn get_price_in_quote(&self, quote: Price, result_expo: i32) -> Option<PriceConf> {
    return match (self.get_current_price(), quote.get_current_price()) {
      (Some(base_price_conf), Some(quote_price_conf)) =>
        base_price_conf.div(quote_price_conf, result_expo),
      (_, _) => None,
    }
  }

  /**
   * Get the time-weighted average price (TWAP) and a confidence interval on the result.
   * Returns None if the twap is currently unavailable.
   *
   * At the moment, the confidence interval returned by this method is computed in
   * a somewhat questionable way, so we do not recommend using it for high-value applications.
   */
  pub fn get_twap(&self) -> Option<PriceConf> {
    // This method currently cannot return None, but may do so in the future.
    // Note that the twac is a positive number in i64, so safe to cast to u64.
    Some(PriceConf { price: self.twap.val, conf: self.twac.val as u64, expo: self.expo })
  }
}


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
#[derive(PartialEq)]
#[derive(Debug)]
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
   * `result_expo` determines the exponent of the result. The minimum possible exponent is
   * `-9 + self.exponent - other.exponent`. (This minimum exponent reflects the maximum possible
   * precision for the result given the precision of the two inputs and the numeric representation.)
   * TODO: possibly allow smaller exponents
   *
   * This function will return `None` unless all of the following conditions are satisfied:
   * 1. The prices of self and other are > 0.
   * 2. The resulting price and confidence can be represented using a 64-bit number with
   *    exponent `result_expo`.
   * 3. The confidence interval of self / other are << MAX_PD_V_U64 times their the respective price.
   *    (This condition should essentially always be satisfied in the real world.)
   */
  pub fn div(&self, other: PriceConf, result_expo: i32) -> Option<PriceConf> {
    // PriceConf is not guaranteed to store its price/confidence in normalized form.
    // Normalize them here to bound the range of price/conf, which is required to perform
    // arithmetic operations.
    match (self.normalized(), other.normalized()) {
      (Some(base), Some(other)) => {
        // Note that normalization implies that the prices can be cast to u64.
        // We need prices as u64 in order to divide, as solana doesn't implement signed division.
        // It's also extremely unclear what this method should do if one of the prices is negative,
        // so assuming positive prices throughout seems fine.

        // These use at most 27 bits each
        let base_price = base.price as u64;
        let other_price = other.price as u64;

        println!("----");
        println!("base ({} +- {}) * 10^{}", base_price, base.conf, base.expo);
        println!("other ({} +- {}) * 10^{}", other_price, other.conf, other.expo);

        // Compute the midprice, base in terms of other.
        // Uses at most 57 bits
        let midprice = base_price * PD_SCALE / other_price;
        let midprice_expo = PD_EXPO + base.expo - other.expo;
        println!("mean {} * 10^{}", midprice, midprice_expo);

        // Compute the confidence interval.
        // This code uses the 1-norm instead of the 2-norm for computational reasons.
        // The correct formula is midprice * sqrt(c_1^2 + c_2^2), where c_1 and c_2 are the
        // confidence intervals in price-percentage terms of the base and other. This quantity
        // is difficult to compute due to the sqrt, and overflow/underflow considerations.
        // Instead, this code uses midprice * (c_1 + c_2).
        // This quantity is at most a factor of sqrt(2) greater than the correct result, which
        // shouldn't matter considering that confidence intervals are typically ~0.1% of the price.

        // The exponent is PD_EXPO for both of these. Each of these uses 57 bits.
        let base_confidence_pct: u64 = (base.conf * PD_SCALE) / base_price;
        let other_confidence_pct: u64 = (other.conf * PD_SCALE) / other_price;

        // at most 58 bits
        let confidence_pct = base_confidence_pct + other_confidence_pct;
        println!("rescaled_z {} * 10^{}", confidence_pct, PD_EXPO);
        // at most 115 bits
        let conf = (confidence_pct as u128) * (midprice as u128);
        let conf_expo = PD_EXPO + midprice_expo;
        println!("conf {} * 10^{}", conf, conf_expo);

        // Scale results to the target exponent.
        let midprice_in_result_expo = PriceConf::scale_to_exponent(midprice as u128, midprice_expo, result_expo);
        let conf_in_result_expo = PriceConf::scale_to_exponent(conf, conf_expo, result_expo);
        match (midprice_in_result_expo, conf_in_result_expo) {
          (Some(m), Some(c)) => {
            let m_i64 = m as i64;
            // This should be guaranteed to succeed because midprice uses <= 57 bits
            assert!(m_i64 >= 0);

            Some(PriceConf {
              price: m_i64,
              conf: c,
              expo: result_expo
            })
          }
          (_, _) => None
        }
      }
      (_, _) => None
    }
  }

  /**
   * Get a copy of this struct where the price and confidence
   * have been normalized to be less than `MAX_PD_V_U64`.
   * Returns `None` if `price == 0` before or after normalization.
   */
  pub fn normalized(&self) -> Option<PriceConf> {
    if self.price > 0 {
      // BPF only supports unsigned division
      let mut p: u64 = self.price as u64;
      let mut c: u64 = self.conf;
      let mut e: i32 = self.expo;

      while p > MAX_PD_V_U64 || c > MAX_PD_V_U64 {
        p = p / 10;
        c = c / 10;
        e += 1;
      }

      // Can get p == 0 if confidence is >> price.
      if p > 0 {
        Some(PriceConf {
          price: p as i64,
          conf: c,
          expo: e,
        })
      } else {
        None
      }
    } else {
      None
    }
  }

  /**
   * Scale num so that its exponent is target_expo.
   * This method can only reduce precision, i.e., target_expo must be > current_expo.
   */
  fn scale_to_exponent(
    num: u128,
    current_expo: i32,
    target_expo: i32,
  ) -> Option<u64> {
    let mut delta = target_expo - current_expo;
    let mut res = num;
    if delta >= 0 {
      while delta > 0 {
        res /= 10;
        delta -= 1;
      }

      if res <= (u64::MAX as u128) {
        Some(res as u64)
      } else {
        None
      }
    } else {
      None
    }
  }
}

struct AccKeyU64
{
  pub val: [u64;4]
}

pub fn cast<T>( d: &[u8] ) -> &T {
  let (_, pxa, _) = unsafe { d.align_to::<T>() };
  &pxa[0]
}

impl AccKey
{
  pub fn is_valid( &self ) -> bool  {
    let k8 = cast::<AccKeyU64>( &self.val );
    return k8.val[0]!=0 || k8.val[1]!=0 || k8.val[2]!=0 || k8.val[3]!=0;
  }
}

#[cfg(test)]
mod test {
  use crate::{MAX_PD_V_I64, MAX_PD_V_U64, PriceConf, PD_SCALE, PD_EXPO};

  fn pc(price: i64, conf: u64, expo: i32) -> PriceConf {
    PriceConf {
      price: price,
      conf: conf,
      expo: expo,
    }
  }

  #[test]
  fn test_rebase() {
    fn test_succeeds(
      price1: PriceConf,
      price2: PriceConf,
      result_expo: i32,
      expected: (i64, u64),
    ) {
      let result = price1.div(price2, result_expo);
      assert_eq!(result, Some(pc(expected.0, expected.1, result_expo)));
    }

    fn test_fails(
      price1: PriceConf,
      price2: PriceConf,
      result_expo: i32,
    ) {
      let result = price1.div(price2, result_expo);
      assert_eq!(result, None);
    }

    test_succeeds(pc(1, 1, 0), pc(1, 1, 0), 0, (1, 2));
    test_succeeds(pc(10, 1, 0), pc(1, 1, 0), 0, (10, 11));
    test_succeeds(pc(1, 1, 1), pc(1, 1, 0), 0, (10, 20));
    test_succeeds(pc(1, 1, -8), pc(1, 1, -8), -8, (100_000_000, 200_000_000));
    test_succeeds(pc(1, 1, 0), pc(5, 1, 0), 0, (0, 0));
    test_succeeds(pc(1, 1, 0), pc(5, 1, 0), -1, (2, 2));
    test_succeeds(pc(1, 1, 0), pc(5, 1, 0), -2, (20, 24));
    test_succeeds(pc(1, 1, 0), pc(5, 1, 0), -9, (200_000_000, 240_000_000));

    // Different exponents in the two inputs
    test_succeeds(pc(100, 10, -8), pc(2, 1, -7), -8, (500_000_000, 300_000_000));
    test_succeeds(pc(100, 10, -4), pc(2, 1, 0), -8, (500_000, 300_000));
    test_succeeds(pc(100, 10, -4), pc(2, 1, 0), -4, (50, 30));

    // Test with end range of possible inputs where the output should not lose precision.
    test_succeeds(pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), 0, (1, 2));
    test_succeeds(pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc(1, 1, 0), 0, (MAX_PD_V_I64, 2 * MAX_PD_V_U64));
    test_succeeds(pc(1, MAX_PD_V_U64, 0), pc(1, MAX_PD_V_U64, 0), 0, (1, 2 * MAX_PD_V_U64));
    test_succeeds(pc(MAX_PD_V_I64, MAX_PD_V_U64, 0), pc(1, MAX_PD_V_U64, 0), 0, (MAX_PD_V_I64, MAX_PD_V_U64 * MAX_PD_V_U64 + (MAX_PD_V_I64 as u64)));

    // More realistic inputs (get BTC price in ETH)
    // Note these inputs are not normalized
    let ten_e7: i64 = 10000000;
    let uten_e7: u64 = 10000000;
    test_succeeds(pc(520010 * ten_e7, 310 * uten_e7, -8),
                  pc(38591 * ten_e7, 18 * uten_e7, -8),
                  -8, (1347490347, 1431804));

    // Test with end range of possible inputs to identify overflow
    // These inputs will lose precision due to the initial normalization.
    // Get the rounded versions of these inputs in order to compute the expected results.
    let normed = pc(i64::MAX, u64::MAX, 0).normalized().unwrap();
    let max_i64 = normed.price * (10_i64.pow(normed.expo as u32));
    let max_u64 = normed.conf * (10_u64.pow(normed.expo as u32));

    test_succeeds(pc(i64::MAX, u64::MAX, 0), pc(i64::MAX, u64::MAX, 0), 0, (1, 4));
    test_succeeds(pc(i64::MAX, u64::MAX, 0), pc(1, 1, 0), 7, (max_i64 / ten_e7, 3 * ((max_i64 as u64) / uten_e7)));

    // Price is zero pre-normalization
    test_fails(pc(0, 1, 0), pc(1, 1, 0), PD_EXPO - 1);
    test_fails(pc(1, 1, 0), pc(0, 1, 0), PD_EXPO - 1);

    // Can't normalize the input when the confidence is >> price.
    test_fails(pc(1, 1, 0), pc(1, u64::MAX, 0), 7);
    test_fails(pc(1, u64::MAX, 0), pc(1, 1, 0), 7);

    // Result exponent too small
    test_succeeds(pc(1, 1, 0), pc(1, 1, 0), PD_EXPO, (1 * (PD_SCALE as i64), 2 * PD_SCALE));
    test_fails(pc(1, 1, 0), pc(1, 1, 0), PD_EXPO - 1);

    // TODO: handle the case where the result exponent is too large more gracefully.
    test_succeeds(pc(1, 1, 0), pc(1, 1, 0), 1, (0, 0));
  }
}