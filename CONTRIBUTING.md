# Contributing

Contributions are welcome for new providers, AOOSTAR display models, layouts and reliability improvements.

1. Fork the repository and create a focused branch.
2. Add or update tests before changing runtime behaviour.
3. Run `python -m unittest discover -s tests -p 'test_aooscope_*.py' -v` and `bash tests/test_deployment.sh`.
4. Do not commit real provider URLs, API keys, machine names, screenshots containing private data, or local `data/` contents.
5. Keep providers optional and failure-isolated: one unreachable service must never stop the display.

Provider code should normalize external APIs into AooScope state rather than leaking service-specific payloads into panel rendering.
