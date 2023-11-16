# policy-reasoner
Implements the famous policy reasoner, known as `checker` in Brane terminology. Builds on top of reasoners like [eFLINT](https://gitlab.com/eflint) and meant to be queried by [Brane](https://github.com/epi-project/brane).


# Needed FIXES

go eFlint server
- struct.go:156 remove omitempty json tag // of niet require empty array
- naming:
    - +user(HospitalA). -> +user(Hospitala).
    - Exists task, dataset' : ... -> Exists task, dataset2 :
- projection:
    - recipient.user == user => (recipient.user) == user
- negation: