local ffi = require('ffi')
ffi.cdef[[char *getenv(const char *name);]]
local mode = ffi.C.getenv('MODE')
print(mode)
