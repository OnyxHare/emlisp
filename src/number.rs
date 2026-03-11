use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

const BASE: u64 = 1_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BigInt {
    sign: i8,
    digits: Vec<u32>,
}

impl BigInt {
    pub fn zero() -> Self {
        Self {
            sign: 0,
            digits: Vec::new(),
        }
    }

    pub fn one() -> Self {
        Self::from_i64(1)
    }

    pub fn from_i64(value: i64) -> Self {
        if value == 0 {
            return Self::zero();
        }
        let sign = if value < 0 { -1 } else { 1 };
        let mut n = value.unsigned_abs();
        let mut digits = Vec::new();
        while n > 0 {
            digits.push((n % BASE) as u32);
            n /= BASE;
        }
        Self { sign, digits }
    }

    pub fn parse_decimal(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }
        let (sign, digits_str) = if let Some(rest) = s.strip_prefix('-') {
            (-1, rest)
        } else if let Some(rest) = s.strip_prefix('+') {
            (1, rest)
        } else {
            (1, s)
        };
        if digits_str.is_empty() || !digits_str.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let mut result = Self::zero();
        for ch in digits_str.chars() {
            let d = (ch as u8 - b'0') as u32;
            result = result.mul_u32(10);
            result = result + Self::from_i64(d as i64);
        }
        if result.is_zero() {
            Some(result)
        } else {
            Some(Self {
                sign,
                digits: result.digits,
            })
        }
    }

    pub fn is_zero(&self) -> bool {
        self.sign == 0
    }

    fn normalize(mut self) -> Self {
        while matches!(self.digits.last(), Some(0)) {
            self.digits.pop();
        }
        if self.digits.is_empty() {
            self.sign = 0;
        }
        self
    }

    fn abs_cmp(&self, other: &Self) -> Ordering {
        if self.digits.len() != other.digits.len() {
            return self.digits.len().cmp(&other.digits.len());
        }
        for (a, b) in self.digits.iter().zip(other.digits.iter()).rev() {
            if a != b {
                return a.cmp(b);
            }
        }
        Ordering::Equal
    }

    fn add_abs(&self, other: &Self) -> Vec<u32> {
        let max_len = self.digits.len().max(other.digits.len());
        let mut out = Vec::with_capacity(max_len + 1);
        let mut carry: u64 = 0;
        for i in 0..max_len {
            let a = *self.digits.get(i).unwrap_or(&0) as u64;
            let b = *other.digits.get(i).unwrap_or(&0) as u64;
            let sum = a + b + carry;
            out.push((sum % BASE) as u32);
            carry = sum / BASE;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        out
    }

    fn sub_abs(&self, other: &Self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.digits.len());
        let mut borrow: i64 = 0;
        for i in 0..self.digits.len() {
            let a = self.digits[i] as i64 - borrow;
            let b = *other.digits.get(i).unwrap_or(&0) as i64;
            if a < b {
                out.push((a + BASE as i64 - b) as u32);
                borrow = 1;
            } else {
                out.push((a - b) as u32);
                borrow = 0;
            }
        }
        out
    }

    fn mul_u32(&self, rhs: u32) -> Self {
        if self.is_zero() || rhs == 0 {
            return Self::zero();
        }
        let mut out = Vec::with_capacity(self.digits.len() + 1);
        let mut carry: u64 = 0;
        for d in &self.digits {
            let prod = *d as u64 * rhs as u64 + carry;
            out.push((prod % BASE) as u32);
            carry = prod / BASE;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        Self {
            sign: self.sign,
            digits: out,
        }
        .normalize()
    }

    pub fn pow10(exp: usize) -> Self {
        let mut n = Self::one();
        for _ in 0..exp {
            n = n.mul_u32(10);
        }
        n
    }
}

impl Add for BigInt {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if self.is_zero() {
            return rhs;
        }
        if rhs.is_zero() {
            return self;
        }
        if self.sign == rhs.sign {
            return Self {
                sign: self.sign,
                digits: self.add_abs(&rhs),
            }
            .normalize();
        }

        match self.abs_cmp(&rhs) {
            Ordering::Greater => Self {
                sign: self.sign,
                digits: self.sub_abs(&rhs),
            }
            .normalize(),
            Ordering::Less => Self {
                sign: rhs.sign,
                digits: rhs.sub_abs(&self),
            }
            .normalize(),
            Ordering::Equal => Self::zero(),
        }
    }
}

impl Sub for BigInt {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Neg for BigInt {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.sign = -self.sign;
        self
    }
}

impl Mul for BigInt {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        if self.is_zero() || rhs.is_zero() {
            return Self::zero();
        }
        let mut out = vec![0u64; self.digits.len() + rhs.digits.len()];
        for (i, a) in self.digits.iter().enumerate() {
            let mut carry = 0u64;
            for (j, b) in rhs.digits.iter().enumerate() {
                let idx = i + j;
                let cur = out[idx] + *a as u64 * *b as u64 + carry;
                out[idx] = cur % BASE;
                carry = cur / BASE;
            }
            if carry > 0 {
                out[i + rhs.digits.len()] += carry;
            }
        }
        let digits = out.into_iter().map(|d| d as u32).collect();
        Self {
            sign: self.sign * rhs.sign,
            digits,
        }
        .normalize()
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.sign.cmp(&other.sign) {
            Ordering::Equal => match self.sign {
                0 => Ordering::Equal,
                1 => self.abs_cmp(other),
                -1 => other.abs_cmp(self),
                _ => unreachable!(),
            },
            ord => ord,
        }
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return write!(f, "0");
        }
        if self.sign < 0 {
            write!(f, "-")?;
        }
        let mut it = self.digits.iter().rev();
        if let Some(first) = it.next() {
            write!(f, "{first}")?;
        }
        for d in it {
            write!(f, "{d:09}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Number {
    numer: BigInt,
    denom: BigInt,
}

impl Number {
    pub fn from_i64(v: i64) -> Self {
        Self {
            numer: BigInt::from_i64(v),
            denom: BigInt::one(),
        }
    }

    pub fn from_parts(numer: BigInt, denom: BigInt) -> Option<Self> {
        if denom.is_zero() {
            return None;
        }
        if numer.is_zero() {
            return Some(Self::from_i64(0));
        }
        if denom < BigInt::zero() {
            return Some(Self {
                numer: -numer,
                denom: -denom,
            });
        }
        Some(Self { numer, denom })
    }

    pub fn is_zero(&self) -> bool {
        self.numer.is_zero()
    }

    pub fn is_integer(&self) -> bool {
        self.denom == BigInt::one()
    }

    pub fn reciprocal(&self) -> Option<Self> {
        Self::from_parts(self.denom.clone(), self.numer.clone())
    }

    pub fn numer(&self) -> &BigInt {
        &self.numer
    }

    pub fn denom(&self) -> &BigInt {
        &self.denom
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.numer.clone() * other.denom.clone() == other.numer.clone() * self.denom.clone()
    }
}

impl Eq for Number {}

impl Add for Number {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let numer = self.numer * rhs.denom.clone() + rhs.numer * self.denom.clone();
        let denom = self.denom * rhs.denom;
        Self::from_parts(numer, denom).expect("denominator is always non-zero")
    }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Neg for Number {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            numer: -self.numer,
            denom: self.denom,
        }
    }
}

impl Mul for Number {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let numer = self.numer * rhs.numer;
        let denom = self.denom * rhs.denom;
        Self::from_parts(numer, denom).expect("denominator is always non-zero")
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.numer.clone() * other.denom.clone()).cmp(&(other.numer.clone() * self.denom.clone()))
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_integer() {
            write!(f, "{}", self.numer)
        } else {
            write!(f, "{}/{}", self.numer, self.denom)
        }
    }
}

pub fn parse_number(token: &str) -> Option<Number> {
    if let Some(int) = BigInt::parse_decimal(token) {
        return Some(Number::from_parts(int, BigInt::one())?);
    }

    let (int_part, frac_part) = token.split_once('.')?;
    if int_part.is_empty() || frac_part.is_empty() {
        return None;
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let sign = int_part.starts_with('-');
    let int_digits = if int_part.starts_with('-') || int_part.starts_with('+') {
        &int_part[1..]
    } else {
        int_part
    };
    if int_digits.is_empty() || !int_digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let whole = BigInt::parse_decimal(int_digits)?;
    let frac = BigInt::parse_decimal(frac_part)?;
    let scale = BigInt::pow10(frac_part.len());
    let mut numer = whole * scale.clone() + frac;
    if sign {
        numer = -numer;
    }
    Number::from_parts(numer, scale)
}
