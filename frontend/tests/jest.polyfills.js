// Polyfill for environment variables that might be missing in test environment
const { TextEncoder, TextDecoder } = require('util');

global.TextEncoder = TextEncoder;
global.TextDecoder = TextDecoder;

// Mock crypto if not available
if (!global.crypto) {
  global.crypto = require('crypto').webcrypto;
}

// Mock IndexedDB
const FDBFactory = require('fake-indexeddb/lib/FDBFactory');
const FDBKeyRange = require('fake-indexeddb/lib/FDBKeyRange');

global.indexedDB = new FDBFactory();
global.IDBKeyRange = FDBKeyRange;