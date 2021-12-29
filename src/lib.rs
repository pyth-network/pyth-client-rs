//! A Rust library for consuming price feeds from the [pyth.network](https://pyth.network/) oracle on the Solana network.
//!
//! # Quick Start
//!
//! Get the price from a Pyth price account:
//!
//! ```no_run
//! use pyth_client::{load_price, PriceConf};
//! // solana account data as bytes, either passed to on-chain program or from RPC connection.
//! let account_data: Vec<u8> = vec![];
//! let price_account = load_price( &account_data ).unwrap();
//! // May be None if price is not currently available.
//! let price: PriceConf = price_account.get_current_price().unwrap();
//! println!("price: {} +- {} x 10^{}", price.price, price.conf, price.expo);
//! ```
//!
//!
//!
//!


pub use self::price_conf::PriceConf;
pub use self::error::PythError;

mod entrypoint;
mod error;
mod price_conf;

pub mod processor;
pub mod instruction;

use std::mem::size_of;
use bytemuck::{
  cast_slice, from_bytes, try_cast_slice,
  Pod, PodCastError, Zeroable,
};

solana_program::declare_id!("PythC11111111111111111111111111111111111111");

pub const MAGIC          : u32   = 0xa1b2c3d4;
pub const VERSION_2      : u32   = 2;
pub const VERSION        : u32   = VERSION_2;
pub const MAP_TABLE_SIZE : usize = 640;
pub const PROD_ACCT_SIZE : usize = 512;
pub const PROD_HDR_SIZE  : usize = 48;
pub const PROD_ATTR_SIZE : usize = PROD_ACCT_SIZE - PROD_HDR_SIZE;

// each account has its own type
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub enum PriceStatus
{
  Unknown,
  Trading,
  Halted,
  Auction
}

// ongoing coporate action event - still undergoing dev
#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub enum CorpAction
{
  NoCorpAct
}

// different types of prices associated with a product
#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub enum PriceType
{
  Unknown,
  Price
}

// solana public key
#[derive(Copy, Clone)]
#[repr(C)]
pub struct AccKey
{
  pub val: [u8;32]
}

// Mapping account structure
#[derive(Copy, Clone)]
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

#[cfg(target_endian = "little")]
unsafe impl Zeroable for Mapping {}

#[cfg(target_endian = "little")]
unsafe impl Pod for Mapping {}


// Product account structure
#[derive(Copy, Clone)]
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

#[cfg(target_endian = "little")]
unsafe impl Zeroable for Product {}

#[cfg(target_endian = "little")]
unsafe impl Pod for Product {}

// contributing or aggregate price component
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PriceComp
{
  pub publisher  : AccKey,     // key of contributing quoter
  pub agg        : PriceInfo,  // contributing price to last aggregate
  pub latest     : PriceInfo   // latest contributing price (not in agg.)
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Ema
{
  pub val        : i64,        // current value of ema
  numer          : i64,        // numerator state for next update
  denom          : i64         // denominator state for next update
}

// Price account structure
#[derive(Copy, Clone)]
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

#[cfg(target_endian = "little")]
unsafe impl Zeroable for Price {}

#[cfg(target_endian = "little")]
unsafe impl Pod for Price {}

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

  /**
   * Get the current price of this account in a different quote currency. If this account
   * represents the price of the product X/Z, and `quote` represents the price of the product Y/Z,
   * this method returns the price of X/Y. Use this method to get the price of e.g., mSOL/SOL from
   * the mSOL/USD and SOL/USD accounts.
   *
   * `result_expo` determines the exponent of the result, i.e., the number of digits below the decimal
   * point. This method returns `None` if either the price or confidence are too large to be
   * represented with the requested exponent.
   */
  pub fn get_price_in_quote(&self, quote: &Price, result_expo: i32) -> Option<PriceConf> {
    return match (self.get_current_price(), quote.get_current_price()) {
      (Some(base_price_conf), Some(quote_price_conf)) =>
        base_price_conf.div(&quote_price_conf)?.scale_to_exponent(result_expo),
      (_, _) => None,
    }
  }

  /**
   * Get the price of a basket of currencies. Each entry in `amounts` is of the form
   * `(price, qty, qty_expo)`, and the result is the sum of `price * qty * 10^qty_expo`.
   * The result is returned with exponent `result_expo`.
   *
   * An example use case for this function is to get the value of an LP token.
   */
  pub fn price_basket(amounts: &[(Price, i64, i32)], result_expo: i32) -> Option<PriceConf> {
    assert!(amounts.len() > 0);
    let mut res = PriceConf { price: 0, conf: 0, expo: result_expo };
    for i in 0..amounts.len() {
      res = res.add(
        &amounts[i].0.get_current_price()?.cmul(amounts[i].1, amounts[i].2)?.scale_to_exponent(result_expo)?
      )?
    }
    Some(res)
  }
}

#[derive(Copy, Clone)]
struct AccKeyU64
{
  pub val: [u64;4]
}

#[cfg(target_endian = "little")]
unsafe impl Zeroable for AccKeyU64 {}

#[cfg(target_endian = "little")]
unsafe impl Pod for AccKeyU64 {}

impl AccKey
{
  pub fn is_valid( &self ) -> bool  {
    match load::<AccKeyU64>( &self.val ) {
      Ok(k8) => k8.val[0]!=0 || k8.val[1]!=0 || k8.val[2]!=0 || k8.val[3]!=0,
      Err(_) => false,
    }
  }
}

fn load<T: Pod>(data: &[u8]) -> Result<&T, PodCastError> {
  let size = size_of::<T>();
  if data.len() >= size {
    Ok(from_bytes(cast_slice::<u8, u8>(try_cast_slice(
      &data[0..size],
    )?)))
  } else {
    Err(PodCastError::SizeMismatch)
  }
}

pub fn load_mapping(data: &[u8]) -> Result<&Mapping, PythError> {
  let pyth_mapping = load::<Mapping>(&data).map_err(|_| PythError::InvalidAccountData)?;

  if pyth_mapping.magic != MAGIC {
    return Err(PythError::InvalidAccountData);
  }
  if pyth_mapping.ver != VERSION_2 {
    return Err(PythError::BadVersionNumber);
  }
  if pyth_mapping.atype != AccountType::Mapping as u32 {
    return Err(PythError::WrongAccountType);
  }

  return Ok(pyth_mapping);
}

pub fn load_product(data: &[u8]) -> Result<&Product, PythError> {
  let pyth_product = load::<Product>(&data).map_err(|_| PythError::InvalidAccountData)?;

  if pyth_product.magic != MAGIC {
    return Err(PythError::InvalidAccountData);
  }
  if pyth_product.ver != VERSION_2 {
    return Err(PythError::BadVersionNumber);
  }
  if pyth_product.atype != AccountType::Product as u32 {
    return Err(PythError::WrongAccountType);
  }

  return Ok(pyth_product);
}

pub fn load_price(data: &[u8]) -> Result<&Price, PythError> {
  let pyth_price = load::<Price>(&data).map_err(|_| PythError::InvalidAccountData)?;

  if pyth_price.magic != MAGIC {
    return Err(PythError::InvalidAccountData);
  }
  if pyth_price.ver != VERSION_2 {
    return Err(PythError::BadVersionNumber);
  }
  if pyth_price.atype != AccountType::Price as u32 {
    return Err(PythError::WrongAccountType);
  }

  return Ok(pyth_price);
}
