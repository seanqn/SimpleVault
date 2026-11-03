"""Regenerate the public test fixture using the legacy libsodium C API."""
import ctypes as c
import ctypes.util
from pathlib import Path
s = c.CDLL(ctypes.util.find_library('sodium'))
assert s.sodium_init() >= 0
message = 'secret🔐'.encode()
output = c.create_string_buffer(len(message) + 16)
length = c.c_ulonglong()
assert s.crypto_aead_xchacha20poly1305_ietf_encrypt(output, c.byref(length), message, c.c_ulonglong(len(message)), None, c.c_ulonglong(0), None, bytes([9])*24, bytes([7])*32) == 0
Path(__file__).resolve().parents[1].joinpath('rust/vault-core/src/fixture.bin').write_bytes(output.raw[:length.value])
