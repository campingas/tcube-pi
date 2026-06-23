# Authentication Guide

T-Cube admin authentication is local to the Mac mini. Accounts use unique usernames and optional display names; email and external identity providers are not required.

## Initial Owner

On a fresh installation, the first page visit shows Create Local Owner before Setup Params. The owner chooses a username, display name, and password, then continues to cube naming and Wi-Fi setup. Existing installations migrate the legacy setup credential to username `admin` as the owner of the setup-linked cube; the first successful password login accepts the legacy password hash and immediately upgrades it to scrypt without changing the password.

## Login And Sessions

Admins sign in with a username and password. Sessions use random tokens stored only as SHA-256 hashes in SQLite and sent through HTTP-only `SameSite=Strict` cookies. Sessions expire after 90 days of inactivity and are renewed during authenticated use. Logout revokes the current session. Password reset with a recovery code revokes every existing session for the account.

## Roles And Cubes

Each Pi-hosted admin instance manages one cube. `owner` can change sensitive setup, create manager invitations, and perform manager actions. `manager` can manage content for the assigned cube but cannot invite accounts or change owner-sensitive setup.

The admin UI and API no longer expose cube selection. Setup, content, and media administration always target the local cube recorded in `device_setup.device_id`. A person who administers multiple physical cubes should open each cube at its own local URL and sign in to that cube independently.

The Rust `tcube-pi-admin` service enforces the same role boundary: owners are required for manager invitations, cube naming, Wi-Fi verification, and setup completion, while owners and managers can use content listing, button content-mode, upload/recording, activation, trash, and cleanup endpoints.

## Invitations

Owners generate a manager invitation link from the authenticated toolbar and share it manually. Invitation codes are random, stored only as SHA-256 hashes, valid for seven days, restricted to the manager role and one cube, and consumed once when the recipient creates a local username and password.

## Recovery

An authenticated admin can generate one recovery code. Generating a replacement invalidates the previous unused code. Recovery codes are stored only as hashes, expire after 30 days, are consumed once, replace the password, and revoke all existing sessions.

If no recovery code was created before password loss, a local operator with filesystem access can run `just create-admin-recovery-code admin`. The command writes the code to the ignored `data/admin-recovery-code.txt` file with mode `0600` instead of printing the secret in command logs.

Treat invitation links, recovery codes, cube tokens, the SQLite database, and certificate private keys as secrets. Do not commit or log them.
