import struct
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from nuif_package_oracle import PackageOracleError, encode_archive, read_archive


class PackageOracleTests(unittest.TestCase):
    def test_round_trip_uses_mimetype_first_and_bytewise_order(self):
        members = [
            ("mimetype", b"application/nuif+zip"),
            ("blobs/sha256/" + "a" * 64, b"blob"),
            ("document.cbor", b"document"),
            ("manifest.cbor", b"manifest"),
        ]
        encoded = encode_archive(members)
        self.assertEqual(read_archive(encoded), members)
        self.assertEqual(struct.unpack_from("<I", encoded)[0], 0x04034B50)

    def test_rejects_noncanonical_member_order(self):
        members = [("mimetype", b"application/nuif+zip"), ("manifest.cbor", b"m"), ("document.cbor", b"d")]
        encoded = bytearray(encode_archive([members[0], members[2], members[1]]))
        central = encoded.find(b"PK\x01\x02")
        encoded[central + 46 : central + 46 + len("document.cbor")] = b"manifest.cbor"
        with self.assertRaises(PackageOracleError):
            read_archive(bytes(encoded))

    def test_crc_is_checked(self):
        members = [("mimetype", b"application/nuif+zip")]
        encoded = bytearray(encode_archive(members))
        encoded[30] ^= 1
        with self.assertRaises(PackageOracleError):
            read_archive(bytes(encoded))


if __name__ == "__main__":
    unittest.main()
