/*
    Cryptography utilities

    Copyright 2018 by Kzen Networks

    This file is part of Cryptography utilities library
    (https://github.com/KZen-networks/cryptography-utils)

    Cryptography utilities is free software: you can redistribute
    it and/or modify it under the terms of the GNU General Public
    License as published by the Free Software Foundation, either
    version 3 of the License, or (at your option) any later version.

    @license GPL-3.0+ <https://github.com/KZen-networks/cryptography-utils/blob/master/LICENSE>
*/

// Secp256k1 elliptic curve utility functions (se: https://en.bitcoin.it/wiki/Secp256k1).
//
// In Cryptography utilities, we need to manipulate low level elliptic curve members as Point
// in order to perform operation on them. As the library secp256k1 expose only SecretKey and
// PublicKey, we extend those with simple codecs.
//
// The Secret Key codec: BigInt <> SecretKey
// The Public Key codec: Point <> SecretKey
//

use BigInt;

use super::rand::{thread_rng, Rng};
use super::secp256k1::constants::{
    CURVE_ORDER, GENERATOR_X, GENERATOR_Y, SECRET_KEY_SIZE, UNCOMPRESSED_PUBLIC_KEY_SIZE,
};
use super::secp256k1::{None, PublicKey, Secp256k1, SecretKey};
use super::traits::{ECPoint, ECScalar};
use arithmetic::traits::{Converter, Modulo};
use cryptographic_primitives::hashing::hash_sha256::HSha256;
use cryptographic_primitives::hashing::traits::Hash;
use serde::de;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::ser::{Serialize, Serializer};
use serde::{Deserialize, Deserializer};
use std::fmt;

pub type EC = Secp256k1<None>;
pub type SK = SecretKey;
pub type PK = PublicKey;

#[derive(Clone, PartialEq, Debug)]
pub struct Secp256k1Scalar {
    purpose: String, // it has to be a non constant string for serialization
    fe: SK,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Secp256k1Point {
    purpose: String, // it has to be a non constant string for serialization
    ge: PK,
}
pub type GE = Secp256k1Point;
pub type FE = Secp256k1Scalar;

impl Secp256k1Point {
    pub fn random_point() -> Secp256k1Point {
        let random_scalar: Secp256k1Scalar = Secp256k1Scalar::new_random();
        let base_point = Secp256k1Point::generator();
        let pk = base_point.scalar_mul(&random_scalar.get_element());
        let mut arr = [0u8; 32];
        thread_rng().fill(&mut arr[..]);
        Secp256k1Point {
            purpose: "random_point".to_string(),
            ge: pk.get_element(),
        }
    }
    //TODO: implement for other curves
    //TODO: make constant
    pub fn base_point2() -> Secp256k1Point {
        let g: Secp256k1Point = ECPoint::generator();
        let hash = HSha256::create_hash(vec![&g.bytes_compressed_to_big_int()]);
        let hash = HSha256::create_hash(vec![&hash]);
        let hash = HSha256::create_hash(vec![&hash]);
        let mut hash_vec = BigInt::to_vec(&hash);
        let mut template: Vec<u8> = vec![2];
        template.append(&mut hash_vec);

        Secp256k1Point {
            purpose: "blind_point".to_string(),
            ge: PK::from_slice(&EC::without_caps(), &template).unwrap(),
        }
    }
}

impl ECScalar<SK> for Secp256k1Scalar {
    fn new_random() -> Secp256k1Scalar {
        let mut arr = [0u8; 32];
        thread_rng().fill(&mut arr[..]);
        Secp256k1Scalar {
            purpose: "random".to_string(),
            //fe: SK::new( & EC::without_caps(), &mut csprng)
            fe: SK::from_slice(&EC::without_caps(), &arr[0..arr.len()]).unwrap(), // fe: SK::new( & EC::without_caps(), &mut thread_rng())
        }
    }

    fn get_element(&self) -> SK {
        self.fe
    }

    fn set_element(&mut self, element: SK) {
        self.fe = element
    }

    fn from(n: &BigInt) -> Secp256k1Scalar {
        let temp_fe: FE = ECScalar::new_random();
        let curve_order = temp_fe.q();
        let n_reduced = BigInt::mod_add(n, &BigInt::from(0), &curve_order);
        let mut v = BigInt::to_vec(&n_reduced);

        if v.len() < SECRET_KEY_SIZE {
            let mut template = vec![0; SECRET_KEY_SIZE - v.len()];
            template.extend_from_slice(&v);
            v = template;
        }
        Secp256k1Scalar {
            purpose: "from_big_int".to_string(),
            fe: SK::from_slice(&EC::without_caps(), &v).unwrap(),
        }
    }

    fn to_big_int(&self) -> BigInt {
        BigInt::from(&(self.fe[0..self.fe.len()]))
    }

    fn q(&self) -> BigInt {
        BigInt::from(CURVE_ORDER.as_ref())
    }

    fn add(&self, other: &SK) -> Secp256k1Scalar {
        let mut other_scalar: FE = ECScalar::new_random();
        other_scalar.set_element(other.clone());
        let res: FE = ECScalar::from(&BigInt::mod_add(
            &self.to_big_int(),
            &other_scalar.to_big_int(),
            &self.q(),
        ));
        Secp256k1Scalar {
            purpose: "add".to_string(),
            fe: res.get_element(),
        }
    }

    fn mul(&self, other: &SK) -> Secp256k1Scalar {
        let mut other_scalar: FE = ECScalar::new_random();
        other_scalar.set_element(other.clone());
        let res: FE = ECScalar::from(&BigInt::mod_mul(
            &self.to_big_int(),
            &other_scalar.to_big_int(),
            &self.q(),
        ));
        Secp256k1Scalar {
            purpose: "mul".to_string(),
            fe: res.get_element(),
        }
    }

    fn sub(&self, other: &SK) -> Secp256k1Scalar {
        let mut other_scalar: FE = ECScalar::new_random();
        other_scalar.set_element(other.clone());
        let res: FE = ECScalar::from(&BigInt::mod_sub(
            &self.to_big_int(),
            &other_scalar.to_big_int(),
            &self.q(),
        ));
        Secp256k1Scalar {
            purpose: "mul".to_string(),
            fe: res.get_element(),
        }
    }
}

impl Serialize for Secp256k1Scalar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_big_int().to_hex())
    }
}

impl<'de> Deserialize<'de> for Secp256k1Scalar {
    fn deserialize<D>(deserializer: D) -> Result<Secp256k1Scalar, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Secp256k1ScalarVisitor)
    }
}

struct Secp256k1ScalarVisitor;

impl<'de> Visitor<'de> for Secp256k1ScalarVisitor {
    type Value = Secp256k1Scalar;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Secp256k1Scalar")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Secp256k1Scalar, E> {
        let v = BigInt::from_str_radix(s, 16).expect("Failed in serde");
        Ok(ECScalar::from(&v))
    }
}

impl ECPoint<PK, SK> for Secp256k1Point {
    fn generator() -> Secp256k1Point {
        let mut v = vec![4 as u8];
        v.extend(GENERATOR_X.as_ref());
        v.extend(GENERATOR_Y.as_ref());
        Secp256k1Point {
            purpose: "base_fe".to_string(),
            ge: PK::from_slice(&Secp256k1::without_caps(), &v).unwrap(),
        }
    }

    fn get_element(&self) -> PK {
        self.ge
    }

    fn x_coor(&self) -> BigInt {
        let serialized_pk = PK::serialize_uncompressed(&self.ge);
        let x = &serialized_pk[1..serialized_pk.len() / 2 + 1];
        BigInt::from(x)
    }

    fn y_coor(&self) -> BigInt {
        let serialized_pk = PK::serialize_uncompressed(&self.ge);
        let y = &serialized_pk[(serialized_pk.len() - 1) / 2 + 1..serialized_pk.len()];
        BigInt::from(y)
    }

    fn bytes_compressed_to_big_int(&self) -> BigInt {
        let serial = self.ge.serialize();
        let result = BigInt::from(&serial[0..33]);
        return result;
    }

    fn pk_to_key_slice(&self) -> Vec<u8> {
        let mut v = vec![4 as u8];

        v.extend(BigInt::to_vec(&self.x_coor()));
        v.extend(BigInt::to_vec(&self.x_coor()));
        v
    }

    fn scalar_mul(mut self, fe: &SK) -> Secp256k1Point {
        self.ge
            .mul_assign(&Secp256k1::new(), fe) // we can't use Secp256k1 <None> (EC) in mul_assign
            .expect("Assignment expected");
        self
    }

    fn add_point(&self, other: &PK) -> Secp256k1Point {
        Secp256k1Point {
            purpose: "combine".to_string(),
            ge: self.ge.combine(&EC::without_caps(), other).unwrap(),
        }
    }

    fn from_coor(x: &BigInt, y: &BigInt) -> Secp256k1Point {
        let mut vec_x = BigInt::to_vec(x);
        let mut vec_y = BigInt::to_vec(y);
        let coor_size = (UNCOMPRESSED_PUBLIC_KEY_SIZE - 1) / 2;

        if vec_x.len() < coor_size {
            // pad
            let mut x_buffer = vec![0; coor_size - vec_x.len()];
            x_buffer.extend_from_slice(&vec_x);
            vec_x = x_buffer
        }

        if vec_y.len() < coor_size {
            // pad
            let mut y_buffer = vec![0; coor_size - vec_y.len()];
            y_buffer.extend_from_slice(&vec_y);
            vec_y = y_buffer
        }

        assert_eq!(x, &BigInt::from(vec_x.as_ref()));
        assert_eq!(y, &BigInt::from(vec_y.as_ref()));

        let mut v = vec![4 as u8];
        v.extend(vec_x);
        v.extend(vec_y);

        Secp256k1Point {
            purpose: "base_fe".to_string(),
            ge: PK::from_slice(&Secp256k1::without_caps(), &v).unwrap(),
        }
    }
}

impl Serialize for Secp256k1Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Secp256k1Point", 2)?;
        state.serialize_field("x", &self.x_coor().to_hex())?;
        state.serialize_field("y", &self.y_coor().to_hex())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Secp256k1Point {
    fn deserialize<D>(deserializer: D) -> Result<Secp256k1Point, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(Secp256k1PointVisitor)
    }
}

struct Secp256k1PointVisitor;

impl<'de> Visitor<'de> for Secp256k1PointVisitor {
    type Value = Secp256k1Point;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Secp256k1Point")
    }

    fn visit_map<E: MapAccess<'de>>(self, mut map: E) -> Result<Secp256k1Point, E::Error> {
        let mut x = String::new();
        let mut y = String::new();

        while let Some(key) = map.next_key::<&'de str>()? {
            let v = map.next_value::<&'de str>()?;
            match key.as_ref() {
                "x" => x = String::from(v),
                "y" => y = String::from(v),
                _ => panic!("Serialization failed!"),
            }
        }

        let bx = BigInt::from_hex(&x);
        let by = BigInt::from_hex(&y);

        Ok(Secp256k1Point::from_coor(&bx, &by))
    }
}

#[cfg(test)]
mod tests {
    use super::BigInt;
    use super::Secp256k1Point;
    use super::Secp256k1Scalar;
    use arithmetic::traits::Converter;
    use elliptic::curves::traits::ECPoint;
    use elliptic::curves::traits::ECScalar;
    use serde_json;

    #[test]
    fn serialize_sk() {
        let scalar: Secp256k1Scalar = ECScalar::from(&BigInt::from(123456));
        let s = serde_json::to_string(&scalar).expect("Failed in serialization");
        assert_eq!(s, "\"1e240\"");
    }

    #[test]
    fn serialize_rand_pk_verify_pad() {
        let vx = BigInt::from_hex(
            &"ccaf75ab7960a01eb421c0e2705f6e84585bd0a094eb6af928c892a4a2912508".to_string(),
        );

        let vy = BigInt::from_hex(
            &"e788e294bd64eee6a73d2fc966897a31eb370b7e8e9393b0d8f4f820b48048df".to_string(),
        );

        Secp256k1Point::from_coor(&vx, &vy); // x and y of size 32

        let x = BigInt::from_hex(
            &"5f6853305467a385b56a5d87f382abb52d10835a365ec265ce510e04b3c3366f".to_string(),
        );

        let y = BigInt::from_hex(
            &"b868891567ca1ee8c44706c0dc190dd7779fe6f9b92ced909ad870800451e3".to_string(),
        );

        Secp256k1Point::from_coor(&x, &y); // x and y not of size 32 each

        let r = Secp256k1Point::random_point();
        let r_expected = Secp256k1Point::from_coor(&r.x_coor(), &r.y_coor());

        assert_eq!(r.x_coor(), r_expected.x_coor());
        assert_eq!(r.y_coor(), r_expected.y_coor());
    }

    #[test]
    fn deserialize_sk() {
        let s = "\"1e240\"";
        let dummy: Secp256k1Scalar = serde_json::from_str(s).expect("Failed in serialization");

        let sk: Secp256k1Scalar = ECScalar::from(&BigInt::from(123456));

        assert_eq!(dummy, sk);
    }

    #[test]
    fn serialize_pk() {
        let pk = Secp256k1Point::generator();
        let x = pk.x_coor();
        let y = pk.y_coor();
        let s = serde_json::to_string(&pk).expect("Failed in serialization");

        let expected = format!("{{\"x\":\"{}\",\"y\":\"{}\"}}", x.to_hex(), y.to_hex());
        assert_eq!(s, expected);

        let des_pk: Secp256k1Point = serde_json::from_str(&s).expect("Failed in serialization");
        assert_eq!(des_pk.ge, pk.ge);
    }
}
