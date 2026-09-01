# Security policy

If you find a vulnerability in mneme (for example unsafe handling of vault files, path traversal, or journal/index corruption that could lose user data), **do not** open a public issue.

Use GitHub's private vulnerability reporting on this repository, or open a security advisory draft.

Please include:

- mneme version / commit
- OS
- Steps to reproduce
- Impact (data loss, lock bypass, unexpected writes outside the vault)

This project indexes local Markdown. Treat the vault as user data: patches must not log note bodies at default log levels.
