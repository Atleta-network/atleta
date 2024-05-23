use sp_core::{ecdsa::Signature, H160, H256, U256};
use sp_std as std;

type Bytes = Vec<u8>;

fn keccak_256(data: &[u8]) -> H256 {
    H256::from_slice(&sp_core::keccak_256(data))
}

pub struct Member {
    name: &'static str,
    type_: &'static str,
}

pub trait Typeable {
    fn type_name() -> &'static str;
    fn members(&self) -> Vec<Member>;

    fn encode_type(&self) -> Bytes {
        let type_name = Self::type_name();

        let members = self
            .members()
            .into_iter()
            .map(|Member { name, type_ }| format!("{type_} {name}")) // TODO: de-alloc
            .collect::<Vec<_>>()
            .join(",");

        // TODO: append here recursive types definition
        // https://eips.ethereum.org/EIPS/eip-712#definition-of-encodetype
        format!("{type_name}({members})").as_bytes().to_vec()
    }
}

pub trait HashableStruct: Typeable {
    fn encode_data(&self) -> Bytes;

    fn hash_struct(&self) -> H256 {
        let encoded_type = self.encode_type();
        let type_hash = keccak_256(&encoded_type);

        let encoded_data = self.encode_data();

        let mut buf = Vec::with_capacity(type_hash.as_bytes().len() + encoded_data.len());
        buf.extend_from_slice(type_hash.as_bytes());
        buf.extend_from_slice(&encoded_data);

        keccak_256(&buf)
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
#[cfg_attr(test, serde(rename_all_fields = "camelCase"))]
pub struct Domain {
    pub name: String,
    pub version: String,
    pub chain_id: U256,
    pub verifying_contract: H160, // 0x0 for native or?
                                  // pub salt: Option<H256>,
}

impl Typeable for Domain {
    fn type_name() -> &'static str {
        "EIP712Domain"
    }
    fn members(&self) -> Vec<Member> {
        vec![
            Member { name: "name", type_: "string" },
            Member { name: "version", type_: "string" },
            Member { name: "chainId", type_: "uint256" },
            Member { name: "verifyingContract", type_: "address" },
        ]
    }
}

impl HashableStruct for Domain {
    fn encode_data(&self) -> Bytes {
        let mut bytes = Vec::with_capacity(4 * 32); // TODO: members.len()

        bytes.extend_from_slice(keccak_256(self.name.as_bytes()).as_bytes());
        bytes.extend_from_slice(keccak_256(self.version.as_bytes()).as_bytes());

        let mut buf = [0u8; 32];

        self.chain_id.to_big_endian(&mut buf);
        bytes.extend_from_slice(&buf);

        buf.fill(0);
        buf[12..].copy_from_slice(self.verifying_contract.as_bytes());
        bytes.extend_from_slice(&buf);

        bytes
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
#[cfg_attr(test, serde(rename_all_fields = "camelCase"))]
pub struct Payload {
    pub sender: H160,
    pub nonce: U256,
    pub call: Vec<u8>,
}

impl Typeable for Payload {
    fn type_name() -> &'static str {
        "Payload"
    }
    fn members(&self) -> Vec<Member> {
        vec![
            Member { name: "sender", type_: "address" },
            Member { name: "nonce", type_: "uint256" },
            Member { name: "call", type_: "bytes" },
        ]
    }
}

impl HashableStruct for Payload {
    fn encode_data(&self) -> Bytes {
        let mut bytes = Vec::with_capacity(3 * 32);

        let mut buf = [0u8; 32];
        buf[12..].copy_from_slice(self.sender.as_bytes());
        bytes.extend_from_slice(&buf);

        // buf.fill(0);
        self.nonce.to_big_endian(&mut buf);
        bytes.extend_from_slice(&buf);

        bytes.extend_from_slice(keccak_256(&self.call).as_bytes());

        bytes
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct TypedData<T> {
    // pub types: Vec<(&'static str, Vec<Member>)>,
    // pub primary_type: &'static str,
    pub domain: Domain,
    pub message: T,
}

impl<T> TypedData<T> {
    pub fn new(domain: Domain, message: T) -> Self {
        Self { domain, message }
    }

    pub fn message_hash(&self) -> H256
    where
        T: HashableStruct,
    {
        let domain_separator = self.domain.hash_struct();
        let message_hash = self.message.hash_struct();

        const PRE: &'static [u8] = b"\x19\x01";

        let mut bytes = Vec::with_capacity(
            PRE.len() + std::mem::size_of::<H256>() + std::mem::size_of::<H256>(),
        );
        bytes.extend_from_slice(PRE);
        bytes.extend_from_slice(domain_separator.as_bytes());
        bytes.extend_from_slice(message_hash.as_bytes());

        keccak_256(&bytes)
    }
}

#[derive(Debug)]
pub enum SignatureParseError {
    InvalidFormat(String),
    InvalidChars(std::str::Utf8Error),
    InvalidHex(std::num::ParseIntError),
}

pub fn parse_signature(hex: &str) -> Result<Signature, SignatureParseError> {
    if hex.len() != 132 {
        return Err(SignatureParseError::InvalidFormat(hex.to_string()));
    }
    let sh = match hex.strip_prefix("0x") {
        Some(sh) if sh.len() == 130 => sh,
        _ => return Err(SignatureParseError::InvalidFormat(hex.to_string())),
    };

    let mut bytes = [0u8; 65]; // r: 32, s: 32, v: 1
    for (i, chunk) in sh.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).map_err(SignatureParseError::InvalidChars)?;
        bytes[i] = u8::from_str_radix(s, 16).map_err(SignatureParseError::InvalidHex)?;
    }

    Ok(Signature::from_raw(bytes))
}

pub fn recover(signature: Signature, data_hash: H256) -> Option<H160> {
    let public = signature.recover_prehashed(data_hash.as_fixed_bytes())?;
    let hash = sp_core::keccak_256(public.as_ref());
    Some(H160::from_slice(&hash[12..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn it_works() {
        let domain = Domain {
            name: "ATLA".to_string(),
            version: "1".to_string(),
            chain_id: 1.into(),
            verifying_contract: H160::zero(),
        };

        let sender = H160::from_str("0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC").unwrap();
        let nonce = 1.into();
        let call = vec![];

        let call = Call { sender, nonce, call };

        let typed_data = TypedData::new(domain, call);

        let message_hash = dbg!(typed_data).message_hash();

        /*
         * const accounts = await ethereum.enable();
         *
         */
    }

    #[test]
    fn it_signs_then_recovers() {
        // TODO: sign the message used sp_core Pair
    }

    #[test]
    #[ignore]
    fn it_signs_verifies_and_recovers_using_external_tools() {
        unimplemented!("yet")
    }
}
