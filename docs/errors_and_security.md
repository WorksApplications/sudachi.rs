# Errors

## When to panic and when to error

Sudachi.rs (as a library) must not panic or crash for any user input.
Those situations should be returned as `Error`s.

Sudachi.rs **can** panic or even produce undefined behavior for invalid binary dictionaries.
There are rudimentary checks on the dictionary load, but the binary dictionaries are mostly trusted.
Debug builds have checks on most operations that can produce an error or UB, 
but some of them do not exist in release builds 
(e.g. connection cost access can do out of bound accesses with invalid dictionaries).

However, producing an invalid binary dictionary is considered a bug.
Additionally, dictionary compilation should validate the csv data as much as possible (producing `Error`s).

It is considered OK if Sudachi.rs panics for any input because of incorrect usage, but it should not be the case
so Sudachi.rs panics only for some inputs.

# Security

Notes on security and error considerations for Sudachi.rs.

## Threat model

It should be safe to use Sudachi.rs with any user-inputted text.
Crashes related to user input will be treated as bugs.

On the other hand, maliciously crafted **binary** dictionaries are not treated as security threats.
In the case of analyzing with possibly unsafe user-provided dictionaries, accept the dictionary in the csv form
and compile it into the binary dictionary instead.
Fortunately, binary dictionaries are mapped read-only and will not allow memory modifications, 
but can allow data leaking in theory.
