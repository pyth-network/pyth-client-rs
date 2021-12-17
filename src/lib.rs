pub const MAGIC          : u32   = 0xa1b2c3d4;
pub const VERSION_2      : u32   = 2;
pub const VERSION        : u32   = VERSION_2;
pub const MAP_TABLE_SIZE : usize = 640;
pub const PROD_ACCT_SIZE : usize = 512;
pub const PROD_HDR_SIZE  : usize = 48;
pub const PROD_ATTR_SIZE : usize = PROD_ACCT_SIZE - PROD_HDR_SIZE;

const PD_EXPO: i32 = -9;
const PD_SCALE: u64 = 1000000000;
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

impl PriceInfo {
  pub fn get_checked(&self) -> Option<(i64, u64)> {
    if !matches!(self.status, PriceStatus::Trading) {
      None
    } else {
      Some((self.price, self.conf))
    }
  }
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
   * Returns a triple of the current price, confidence interval, and the exponent for both
   * numbers. For example:
   *
   * get_current_price() -> Some((12345, 267, -2)) // represents 123.45 +- 2.67
   * get_current_price() -> Some((123, 1, 2)) // represents 12300 +- 100
   *
   * Returns None if price information is currently unavailable.
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
   * The value returned by this method has the same semantics as Price::get_current_price above.
   *
   * `result_expo` determines the exponent of the result, i.e., the number of digits of precision in
   * the price. For any given base/quote pair, the minimum possible exponent is
   * `-9 + base.exponent - quote.exponent`.
   */
  pub fn get_price_in_quote(&self, quote: Price, result_expo: i32) -> Option<PriceConf> {
    return match self.get_current_price() {
      Some(base_price_conf) =>
        match quote.get_current_price() {
          Some(quote_price_conf) =>
            Some(base_price_conf.div(quote_price_conf, result_expo));
          None => None
        }
      None => None
    }
  }

  /**
   * Get the time-weighted average price (TWAP) as a fixed point number of the form a * 10^e.
   * Returns a tuple of the current twap and its exponent. For example:
   *
   * get_twap() -> Some((123, -2)) // represents 1.23
   * get_twap() -> Some((45, 3)) // represents 45000
   *
   * Returns None if the twap is currently unavailable.
   */
  pub fn get_twap(&self) -> Option<(i64, i32)> {
    // This method currently cannot return None, but may do so in the future.
    Some((self.twap.val, self.expo))
  }
}

pub struct PriceConf {
  pub price: i64,
  pub conf: u64,
  pub expo: i32,
}

impl PriceConf {
  /**
   * Divides this price and confidence interval by y.
   *
   * The result is returned with result_exp
   */
  pub fn div(&self, quote: PriceConf, result_expo: i32) -> PriceConf {
    // Note that this assertion implies that the prices can be cast to u64.
    // We need prices as u64 in order to divide, as solana doesn't implement signed division.
    // It's also extremely unclear what this method should do if one of the prices is negative,
    // so assuming positive prices throughout seems fine.
    assert!(self.price >= 0 && self.price <= MAX_PD_V_I64);
    assert!(quote.price > 0 && quote.price <= MAX_PD_V_I64);
    let base_price = self.price as u64;
    let quote_price = quote.price as u64;

    assert!(self.conf <= MAX_PD_V_U64);
    assert!(quote.conf <= MAX_PD_V_U64);

    println!("base ({} +- {}) * 10^{}", base_price, self.conf, self.expo);
    println!("quote ({} +- {}) * 10^{}", quote_price, quote.conf, quote.expo);

    // Compute the midprice, base in terms of quote.
    let midprice = (base_price * PD_SCALE) / quote_price;
    let midprice_expo = PD_EXPO + self.expo - quote.expo;
    println!("mean {} * 10^{}", midprice, midprice_expo);
    assert!(result_expo >= midprice_expo);

    // Compute the confidence interval.
    // This code uses the 1-norm instead of the 2-norm for computational reasons.
    // The correct formula is midprice * sqrt(c_1^2 + c_2^2), where c_1 and c_2 are the
    // confidence intervals in price-percentage terms of the base and quote. This quantity
    // is difficult to compute due to the sqrt, and overflow/underflow considerations.
    // Instead, this code uses midprice * (c_1 + c_2).
    // This quantity is at most a factor of sqrt(2) greater than the correct result, which
    // shouldn't matter considering that confidence intervals are typically ~0.1% of the price.

    // The exponent is PD_EXPO for both of these.
    let base_confidence_pct = (self.conf * PD_SCALE) / base_price;
    let quote_confidence_pct = (quote.conf * PD_SCALE) / quote_price;

    // Need to rescale the numbers to prevent the multiplication from overflowing
    let (rescaled_z, rescaled_z_expo) = rescale_num(base_confidence_pct + quote_confidence_pct, PD_EXPO);
    println!("rescaled_z {} * 10^{}", rescaled_z, rescaled_z_expo);
    let (rescaled_mid, rescaled_mid_expo) = rescale_num(midprice, midprice_expo);
    println!("rescaled_mean {} * 10^{}", rescaled_mid, rescaled_mid_expo);
    let conf = (rescaled_z * rescaled_mid);
    let conf_expo = rescaled_z_expo + rescaled_mid_expo;
    println!("conf {} * 10^{}", conf, conf_expo);

    // Scale results to the target exponent.
    let midprice_in_result_expo = scale_to_exponent(midprice, midprice_expo, result_expo);
    let conf_in_result_expo = scale_to_exponent(conf, conf_expo, result_expo);
    let midprice_i64 = midprice_in_result_expo as i64;
    assert!(midprice_i64 >= 0);

    PriceConf {
      price: midprice_i64,
      conf: conf_in_result_expo,
      expo: result_expo
    }
  }

  /** Scale num and its exponent such that it is < MAX_PD_V_U64
   * (which guarantees that multiplication doesn't overflow).
   */
  fn rescale_num(
    num: u64,
    expo: i32,
  ) -> (u64, i32) {
    let mut p: u64 = num;
    let mut c: i32 = 0;

    while p > MAX_PD_V_U64 {
      p = p / 10;
      c += 1;
    }

    println!("c: {}", c);

    return (p, expo + c);
  }

  /** Scale num so that its exponent is target_expo.
   * This method can only reduce precision, i.e., target_expo must be > current_expo.
   */
  fn scale_to_exponent(
    num: u64,
    current_expo: i32,
    target_expo: i32,
  ) -> u64 {
    let mut delta = target_expo - current_expo;
    let mut res = num;
    assert!(delta >= 0);

    while delta > 0 {
      res /= 10;
      delta -= 1;
    }

    return res;
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
  use crate::{AccountType, CorpAction, MAGIC, MAX_PD_V_I64, MAX_PD_V_U64, Price, PriceInfo, PriceStatus, PriceType, rebase_price_info, VERSION, PriceConf};

  #[test]
  fn test_rebase() {
    fn run_test(
      price1: PriceConf,
      price2: PriceConf,
      result_expo: i32,
      expected: (i64, u64),
    ) {
      let result = pinfo1.div(pinfo2, result_expo);
      assert_eq!(result, Some((expected.0, expected.1, result_expo)));
    }

    run_test(PriceConf(1, 1, 0), PriceConf(1, 1, 0), 0, (1, 2));
    run_test(PriceConf(10, 1, 0), PriceConf(1, 1, 0), 0, (10, 11));
    run_test(PriceConf(1, 1, 1), PriceConf(1, 1, 0), 0, (10, 20));
    run_test(PriceConf(1, 1, 0), PriceConf(5, 1, 0), 0, (0, 0));
    run_test(PriceConf(1, 1, 0), PriceConf(5, 1, 0), -2, (20, 24));

    // Test with end range of possible inputs to check for overflow.
    run_test(PriceConf(MAX_PD_V_I64, MAX_PD_V_U64, 0), PriceConf(MAX_PD_V_I64, MAX_PD_V_U64, 0), 0, (1, 2));
    run_test(PriceConf(MAX_PD_V_I64, MAX_PD_V_U64, 0), PriceConf(1, 1, 0), 0, (MAX_PD_V_I64, 2 * MAX_PD_V_U64));
    run_test(PriceConf(1, MAX_PD_V_U64, 0), PriceConf(1, MAX_PD_V_U64, 0), 0, (1, 2 * MAX_PD_V_U64));

    // TODO: need tests at the edges of the capacity of PD

    // TODO: Test non-trading cases

    // TODO: test cases where the exponents are dramatically different
  }
}