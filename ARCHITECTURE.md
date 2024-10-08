Policy reasoner architecture
============================

The policy reasoner repository consists of two main parts. A library and a set of
implementations.

### Library

The entry point for the library is `src/lib.rs` as per usual.
In these files no particular implementation of a policy reasoner is included, only
the mechanisms on which the concept of a policy reasoner can be implemented.

### Interface

The interface for the different reasoners must be the same. Its command line
arguments are defined in `src/bin/implementation/interface.rs`. It could be that a new
reasoner requires more arguments. You can implement another arguments struct, however
make sure the arguments defined in the common interface are supported as other
components of infrastructure may depend on it.

### Implementations

As of now there are three different implementations of a policy reasoners in
this repository. The entrypoint for a reasoner can be found in `src/bin/{...}.rs`.
The subsequent libraries that support the implementation, can be placed in the
`implementation` module in `src/bin/implementation/{...}`.

#### No-Op reasoner

The most minimal reasoner is called the no_op reasoner, it will always returns the
same verdict. This is not a good policy reasoner, but it can be a minimal
example for futher implementations of new policy reasoners.

#### Posix reasoner

There exists a reasoner that uses posix file permissions for access control. As
of now there exists a yaml mapping between file system uids and users with
regards to the policy reasoner, but the idea is to inject multiple ways of
mapping these users to file system uid / gids by means of LDAP/AD for example.
Right now there is no support for linux ACL (facl), but this could be added in
the future.

#### eFLINT reasoner

Lastly, there is a reasoner that implements the eFLINT language as a method of
expressing the control policies. This language can be very expressive with
regards to possible policies, but can also be more complicated to set up.

#### Extra reasonsers

We are looking for new and different ways of expressing policies. If you have an
idea on an implementation for a policy reasoner, feel free to open an issue and
we can discuss how to go about implementing it.
