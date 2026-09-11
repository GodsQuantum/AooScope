# Security policy

## Supported version

Security fixes target the current `latest` image and the current `main` branch.

## Deployment model

AooScope is an administrative dashboard. It can store API credentials for services you configure and can write to the AOOSTAR LCD device. The built-in web UI intentionally has **no authentication layer**; the public Compose binds to `127.0.0.1` by default.

Do not expose port `8765` directly to an untrusted network. Use a VPN or an authenticated reverse proxy if remote access is required.

Provider credentials are written separately from public settings under `/app/cfg/private/providers.json` with mode `0600`. API responses expose only whether a secret exists, never its value. TLS verification is enabled by default and should only be disabled for trusted private endpoints with self-signed certificates.

The container does not require `privileged: true`, the Docker socket, host networking, or host filesystem access. It receives only the LCD device and its dedicated configuration directory.

## Reporting

Please use GitHub private vulnerability reporting when available. Do not include real API keys, private URLs or server inventories in public issues.
