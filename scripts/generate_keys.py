# a script to generate keys for technical accounts as well as session keys
# from a given mnemonic:
# python generate_keys.py <mnemonic>

import hashlib, binascii, argparse, subprocess, re, sys
from hashlib import pbkdf2_hmac
from eth_keys import keys
import nacl.signing
import nacl.encoding


def main():
    mnemonic = parse_args().mnemonic
    names = [
        "lionel", # root
        "diego",
        "diego//stash",
        "pele",
        "pele//stash",
        "franz",
        "franz//stash",
        "johan",
        "ronaldo",
        "zinedine",
        "cristiano",
        "michel",
        "roberto",
    ]
    generate_accounts(mnemonic, names)
    generate_session_keys(mnemonic, ["diego", "pele", "franz"])


def parse_args():
    parser = argparse.ArgumentParser(description='generate keys for technical accounts')
    parser.add_argument('-m', '--mnemonic', type=str, help='the core mnemonic phrase from which the keys will be derived')
    args = parser.parse_args()
    assert args.mnemonic, "mnemonic must be specified"
    return args


def generate_accounts(mnemonic, names):
    for name in names:
        derived_mnemonic = f"{mnemonic}//{name}"
        seed = mnemonic_to_seed(derived_mnemonic)
        address = seed_to_eth_address(seed[:32])
        
        print(f"# {name}")
        print(f"Private: {seed_to_hex(seed)}")
        print(f"Address: {address}")
        print("\n")


def generate_session_keys(mnemonic, names):
    for name in names:
        print(f"# {name} Session Keys:\n")
        
        print("# BABE")
        generate_session_key(mnemonic, name, "babe", "sr25519")
        print("\n")
        
        print("# GRAN (GRANDPA)")
        generate_session_key(mnemonic, name, "gran", "ed25519")
        print("\n")

        print("# IMON (I'm Online)")
        generate_session_key(mnemonic, name, "imon", "sr25519")
        print("\n")
        

def generate_session_key(mnemonic, name, code, scheme):
    command = ["subkey", "inspect", f"{mnemonic}//{name}//{code}", "--scheme", scheme]
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode == 0:
        output = result.stdout
    else:
        print("This is also an error message", file=sys.stderr)
        exit(1)

    secret_seed = get_from_subkey_out("Secret seed", output)
    public_key = get_from_subkey_out("Public key \(hex\)", output)

    print(f"Private: {secret_seed}")
    print(f"Public: {public_key}")


def get_from_subkey_out(key, output):
    pattern = key + r":\s+([0-9a-fx]+)"
    match = re.search(pattern, output)

    if match:
        return match.group(1)
    else:
        print(f"{key} not found")
        exit(1)


def mnemonic_to_seed(mnemonic, passphrase=''):
    # Normalize the mnemonic phrase
    mnemonic_normalized = mnemonic.encode('utf-8')
    
    # Prepare the salt. "mnemonic" is appended to the passphrase as per BIP-39
    salt = ('mnemonic' + passphrase).encode('utf-8')
    
    # Generate the seed using PBKDF2 HMAC-SHA512
    seed = pbkdf2_hmac('sha512', mnemonic_normalized, salt, 2048, dklen=32)
    
    return seed


def seed_to_eth_address(seed):
    private_key = keys.PrivateKey(seed)
    public_key = private_key.public_key
    address = public_key.to_checksum_address()
    
    return address


def seed_to_hex(seed):
    return "0x" + binascii.hexlify(seed).decode('utf-8')


if __name__ == "__main__":
    main()
