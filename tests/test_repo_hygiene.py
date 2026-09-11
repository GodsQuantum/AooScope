import subprocess
import unittest
from pathlib import Path


class PublicRepoHygieneTests(unittest.TestCase):
    def test_repository_has_no_local_installation_identifiers(self):
        forbidden = [
            "Cloud" + " 9", "cloud" + "9", "192." + "168.1.",
            "CT" + "130", "Arez" + "ki", "/srv/lxc/" + "administration",
        ]
        proc = subprocess.run(
            ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
            check=True, text=True, capture_output=True,
        )
        hits = []
        for raw in proc.stdout.splitlines():
            path = Path(raw)
            if not path.is_file() or '.git' in path.parts:
                continue
            try:
                text = path.read_text(encoding='utf-8')
            except (UnicodeDecodeError, OSError):
                continue
            for needle in forbidden:
                if needle.lower() in text.lower() or needle.lower() in raw.lower():
                    hits.append(f"{raw}: {needle}")
        self.assertEqual(hits, [], "local installation data leaked into publishable files:\n" + "\n".join(hits))


if __name__ == '__main__':
    unittest.main()
