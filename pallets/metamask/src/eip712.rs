use sp_core::{H160, H256, U256};
use sp_std::{mem, prelude::*, str, vec::Vec};

const _: () = {
    assert!(mem::size_of::<H256>() == 32);
    assert!(mem::size_of::<U256>() == 32);
};

fn keccak_256(data: impl AsRef<[u8]>) -> H256 {
    H256::from_slice(&sp_io::hashing::keccak_256(data.as_ref()))
}

pub struct Member {
    name: &'static str,
    type_: &'static str,
}

pub trait Typeable {
    fn type_name() -> &'static str;
    // NOTE: probably can't be const as member of optionals are omitted when is none
    fn members(&self) -> Vec<Member>;

    // TODO: append here recursive types definition
    // https://eips.ethereum.org/EIPS/eip-712#definition-of-encodetype
    fn encode_type(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // 'type'
        bytes.extend_from_slice(Self::type_name().as_bytes());

        // 'type(name type,name type)'
        bytes.push(b'(');
        for (i, Member { name, type_ }) in self.members().into_iter().enumerate() {
            if i > 0 {
                bytes.push(b',');
            }
            bytes.extend_from_slice(type_.as_bytes());
            bytes.push(b' ');
            bytes.extend_from_slice(name.as_bytes());
        }
        bytes.push(b')');

        bytes
    }
}

pub trait HashableStruct: Typeable {
    fn encode_data(&self) -> Vec<u8>;

    fn hash_struct(&self) -> H256 {
        let encoded_type = self.encode_type();
        let type_hash = keccak_256(encoded_type);

        let encoded_data = self.encode_data();

        let mut buf = Vec::with_capacity(type_hash.as_bytes().len() + encoded_data.len());
        buf.extend_from_slice(type_hash.as_bytes());
        buf.extend_from_slice(&encoded_data);

        keccak_256(&buf)
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
#[cfg_attr(test, serde(rename_all = "camelCase"))]
pub struct Domain {
    pub name: Vec<u8>,
    pub version: Vec<u8>,
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
    fn encode_data(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 * 32); // TODO: members.len()

        bytes.extend_from_slice(keccak_256(&self.name).as_bytes());
        bytes.extend_from_slice(keccak_256(&self.version).as_bytes());

        let mut buf = [0u8; 32];

        self.chain_id.to_big_endian(&mut buf);
        bytes.extend_from_slice(&buf);

        buf.fill(0);
        buf[12..].copy_from_slice(self.verifying_contract.as_bytes());
        bytes.extend_from_slice(&buf[..]);

        bytes
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
#[cfg_attr(test, serde(rename_all = "camelCase"))]
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
    fn encode_data(&self) -> Vec<u8> {
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

        const PRE: &[u8] = b"\x19\x01";

        let mut bytes =
            Vec::with_capacity(PRE.len() + mem::size_of::<H256>() + mem::size_of::<H256>());
        bytes.extend_from_slice(PRE);
        bytes.extend_from_slice(domain_separator.as_bytes());
        bytes.extend_from_slice(message_hash.as_bytes());

        keccak_256(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::{str::FromStr, *};

    fn hex(bytes: impl AsRef<[u8]>) -> String {
        use std::fmt::Write;
        let bytes = bytes.as_ref();
        let mut s = String::with_capacity(2 * bytes.len());
        for b in bytes {
            write!(&mut s, "{:02x}", b).expect("byte conversion");
        }
        assert_eq!(s.len(), bytes.len() * 2);
        s
    }

    fn btos(bytes: impl AsRef<[u8]>) -> String {
        str::from_utf8(bytes.as_ref()).expect("valid str repr").to_string()
    }

    #[test]
    fn it_matches_hashes() {
        let domain = Domain {
            name: b"ATLA".into(),
            version: b"1".into(),
            chain_id: 1.into(),
            verifying_contract: H160::zero(),
        };

        {
            let t = domain.encode_type();
            println!("domain type: {}", btos(&t));
            println!("domain type hash: {}", hex(keccak_256(t)));
            println!("domain data: {}", hex(domain.encode_data()));
            println!("domain hash: {}", hex(domain.hash_struct()));
        }

        assert_eq!(
            domain.hash_struct(),
            H256::from_str("0xa5440927468f230d9b651709493ed582eb82abe21461b95d0e7dbc5932cac64f")
                .expect("valid hash")
        );

        let payload = Payload {
            sender: H160::from_str("0xcccccccccccccccccccccccccccccccccccccccc").expect("address"),
            nonce: 1.into(),
            call: Vec::new(),
        };

        {
            let t = payload.encode_type();
            println!("payload type: {}", btos(&t));
            println!("payload type hash: {}", hex(keccak_256(t)));
            println!("payload data: {}", hex(payload.encode_data()));
            println!("payload hash: {}", hex(payload.hash_struct()));
        }

        let typed_data = dbg!(TypedData::new(domain, payload));
        // TODO: populate with types for easy use of 'eth_signTypedData_v4'
        // let json = serde_json::to_string(&typed_data).expect("json");
        // println!("{}", json);

        let message_hash = dbg!(typed_data.message_hash());
        assert_eq!(
            message_hash,
            H256::from_str("0x7e972c35e3505118083e81f940180bc7433c78d67edc5a2685d39063464eff80")
                .expect("hash")
        );
    }

    #[test]
    #[ignore]
    fn it_signs_verifies_and_recovers_using_external_tools() {
        // TODO: try one of this:
        // - sign the message used `sp_core::Pair`
        // - call `eth-sig-util` JS package
        unimplemented!("yet")
    }
}
