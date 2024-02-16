# a script to generate keys for technical accounts as well as session keys
# from a given mnemonic:
# python generate_keys.py -m <mnemonic> [-e <envfile>]

import hashlib, binascii, argparse, subprocess, re, sys
from hashlib import pbkdf2_hmac
from eth_keys import keys
import nacl.signing
import nacl.encoding


def main():
    args = parse_args()
    mnemonic = args.mnemonic
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
    accounts = generate_accounts(mnemonic, names)
    if not args.quiet:
        print_accounts(accounts)
    
    session_keys = generate_session_keys(mnemonic, ["diego", "pele", "franz"])
    if not args.quiet:
        print_session_keys(session_keys)

    if args.envfile:
        write_dotenv(accounts, session_keys, args.envfile)


def parse_args():
    parser = argparse.ArgumentParser(description='generate keys for technical accounts')
    parser.add_argument('-q', '--quiet', action="store_true", help="don't print into output")
    parser.add_argument('-m', '--mnemonic', type=str, help='the core mnemonic phrase from which the keys will be derived')
    parser.add_argument('-e', '--envfile', type=str, help='if set, the script will generate a .env file with all the variables at the provided path')
    args = parser.parse_args()
    assert args.mnemonic, "mnemonic must be specified"
    return args


def generate_accounts(mnemonic, names):
    result = []
    for name in names:
        derived_mnemonic = f"{mnemonic}//{name}"
        pair = {}
        pair["name"] = name
        pair["seed"] = mnemonic_to_seed(derived_mnemonic)
        pair["public"] = seed_to_eth_address(pair["seed"][:32])
        pair["seed"] = seed_to_hex(pair["seed"])
        result.append(pair)

    return result


def print_accounts(accounts):
    for account in accounts:
        print(f"# {account['name']}")
        print_pair(account)
        print("\n")

        
def print_pair(pair):            
    print(f"Private: {pair['seed']}")
    print(f"Public: {pair['public']}")
    

def generate_session_keys(mnemonic, names):
    result = []
    for name in names:
        keys = {}
        keys["name"] = name
        keys["babe"] = generate_session_key(mnemonic, name, "babe", "sr25519")
        keys["gran"] = generate_session_key(mnemonic, name, "gran", "ed25519")
        keys["imon"] = generate_session_key(mnemonic, name, "imon", "sr25519")

        result.append(keys)

    return result


def print_session_keys(session_keys):
    for keys in session_keys:
        print(f"# {keys['name']} Session Keys:\n")

        print("# BABE")
        print_pair(keys['babe'])
        print("\n")
        
        print("# GRAN (GRANDPA)")
        print_pair(keys['gran'])
        print("\n")

        print("# IMON (I'm Online)")
        print_pair(keys['imon'])
        print("\n")


def generate_session_key(mnemonic, name, code, scheme):
    command = ["subkey", "inspect", f"{mnemonic}//{name}//{code}", "--scheme", scheme]
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode == 0:
        output = result.stdout
    else:
        print("This is also an error message", file=sys.stderr)
        exit(1)

    pair = {}
    pair["seed"] = get_from_subkey_out("Secret seed", output)
    pair["public"] = get_from_subkey_out("Public key \(hex\)", output)


    return pair


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


def write_dotenv(accounts, session_keys, filepath):
    with open(filepath, 'w') as file:
        file.write("# if you can see this file in a source control system,\n")
        file.write("# the data here should be considered the leaked secrets\n")
        file.write("# and the keys should be updated\n\n\n")

        for account in accounts:
            file.write(f"# {account['name']}\n")
            file.write(f"{account['name'].upper().replace('//', '_')}_PRIVATE=")
            file.write(f'"{account["seed"]}"')
            file.write("\n")

            file.write(f"{account['name'].upper().replace('//', '_')}_PUBLIC=")
            file.write(f'"{account["public"]}"')
            file.write("\n\n")

        file.write("\n# SESSION KEYS\n\n")
        for keys in session_keys:
            file.write(f"# {keys['name']}\n")
            for code in ["babe", "gran", "imon"]:
                file.write(f"{keys['name'].upper().replace('//', '_')}_{code.upper()}_PRIVATE=")
                file.write(f'"{keys[code]["seed"]}"')
                file.write("\n")
                
                file.write(f"{keys['name'].upper().replace('//', '_')}_{code.upper()}_PUBLIC=")
                file.write(f'"{keys[code]["public"]}"')
                file.write("\n")

            file.write("\n")
            
        

if __name__ == "__main__":
    main()
