import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer

from aooscope.media import (
    JSONClient,
    normalize_jelly_latest,
    normalize_jelly_sessions,
    normalize_qbit_torrents,
    normalize_radarr_queue,
    normalize_silo_sessions,
    select_display_event,
    collect_media_state,
    build_media_clients,
)


class AooScopeMediaTests(unittest.TestCase):
    def test_jellyfin_playing_is_poster_first_state(self):
        state = normalize_jelly_sessions([{
            "UserName": "Demo User",
            "NowPlayingItem": {"Id": "m1", "Name": "Dune", "RunTimeTicks": 1000, "ProductionYear": 2026},
            "PlayState": {"PositionTicks": 250, "IsPaused": False},
            "TranscodingInfo": None,
            "Client": "Jellyfin TV",
        }], source="jellyfin", base_url="http://media:8096")
        self.assertEqual(state["mode"], "playing")
        self.assertEqual(state["progress_pct"], 25.0)
        self.assertIn("/Items/m1/Images/Primary", state["poster_url"])

    def test_silo_native_session_has_poster_eta_style_fields(self):
        state = normalize_silo_sessions([{
            "media_title": "Dune",
            "poster_url": "/images/dune.jpg",
            "file_duration": 600,
            "position_seconds": 150,
            "is_paused": False,
            "effective_play_method": "direct",
            "client_label": "Silo TV",
            "source_video_resolution": "2160p",
        }], base_url="http://media:8091")
        self.assertEqual(state["mode"], "playing")
        self.assertEqual(state["progress_pct"], 25.0)
        self.assertEqual(state["quality"], "2160p")
        self.assertEqual(state["poster_url"], "http://media:8091/images/dune.jpg")

    def test_jelly_latest_becomes_landed_visual_state(self):
        state = normalize_jelly_latest([{
            "Id": "new1", "Name": "Alien: Earth", "ProductionYear": 2026,
            "Type": "Movie", "DateCreated": "2026-09-11T07:30:00Z",
        }], base_url="http://media:8096")
        self.assertEqual(state["mode"], "landed")
        self.assertEqual(state["title"], "Alien: Earth")
        self.assertIn("/Items/new1/Images/Primary", state["poster_url"])

    def test_qbit_incoming_formats_eta_and_progress(self):
        state = normalize_qbit_torrents([{
            "name": "Dune.Part.Three.2160p",
            "state": "downloading",
            "progress": 0.73,
            "eta": 754,
            "dlspeed": 42000000,
        }])
        self.assertEqual(state["mode"], "incoming")
        self.assertEqual(state["progress_pct"], 73.0)
        self.assertEqual(state["eta_minutes"], 13)
        self.assertEqual(state["speed_bytes_s"], 42000000)

    def test_radarr_queue_exposes_movie_art_and_eta(self):
        state = normalize_radarr_queue({"records": [{
            "title": "Dune Part Three", "size": 1000, "sizeleft": 400,
            "timeleft": "00:08:30",
            "movie": {"title": "Dune: Part Three", "images": [
                {"coverType": "poster", "remoteUrl": "https://img/poster.jpg"}
            ]},
        }]})
        self.assertEqual(state["title"], "Dune: Part Three")
        self.assertEqual(state["progress_pct"], 60.0)
        self.assertEqual(state["eta_minutes"], 9)
        self.assertEqual(state["poster_url"], "https://img/poster.jpg")

    def test_json_client_sends_configured_header(self):
        seen = {}
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                seen["path"] = self.path
                seen["token"] = self.headers.get("X-Api-Key")
                body = json.dumps({"records": []}).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            def log_message(self, *args):
                pass
        server = HTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            client = JSONClient(f"http://127.0.0.1:{server.server_port}", headers={"X-Api-Key": "sekret"})
            self.assertEqual(client.get("/api/v3/queue", {"pageSize": 5}), {"records": []})
            self.assertTrue(seen["path"].startswith("/api/v3/queue?"))
            self.assertEqual(seen["token"], "sekret")
        finally:
            server.shutdown(); server.server_close()

    def test_event_priority_prefers_playing_then_incoming(self):
        playing = {"mode": "playing", "title": "Film"}
        incoming = {"mode": "incoming", "title": "Download"}
        landed = {"mode": "landed", "title": "New"}
        self.assertEqual(select_display_event(playing, incoming, landed), playing)
        self.assertEqual(select_display_event({"mode": "idle"}, incoming, landed), incoming)
        self.assertEqual(select_display_event({"mode": "idle"}, {"mode": "idle"}, landed), landed)

    def test_build_media_clients_reads_secret_files_without_exposing_values(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as td:
            td=Path(td)
            (td/"jelly").write_text("jelly-secret\n")
            (td/"radarr").write_text("radarr-secret\n")
            env={
                "JELLYFIN_URL":"http://jf:8096",
                "JELLYFIN_API_KEY_FILE":str(td/"jelly"),
                "RADARR_URL":"http://radarr:7878",
                "RADARR_API_KEY_FILE":str(td/"radarr"),
                "QBIT_URL":"http://qbit:8080",
            }
            clients=build_media_clients(env)
            self.assertEqual(clients["jellyfin"].headers["X-Emby-Token"], "jelly-secret")
            self.assertEqual(clients["radarr"].headers["X-Api-Key"], "radarr-secret")
            self.assertEqual(clients["qbittorrent"].headers, {})
            self.assertIsNone(clients.get("silo"))

    def test_collect_media_state_is_offline_safe_and_merges_incoming(self):
        class Fake:
            def __init__(self, payloads): self.payloads=payloads
            def get(self, path, params=None):
                value=self.payloads[path]
                if isinstance(value, Exception): raise value
                return value
        jelly=Fake({
            "/Sessions": [],
            "/Items/Latest": [{"Id":"j1","Name":"New Film","Type":"Movie"}],
        })
        radarr=Fake({"/api/v3/queue":{"records":[{
            "downloadId":"abc","size":100,"sizeleft":20,"timeleft":"00:10:00",
            "movie":{"title":"Film X","images":[{"coverType":"poster","remoteUrl":"https://img/x.jpg"}]}}]}})
        qbit=Fake({"/api/v2/torrents/info":[{
            "hash":"abc","name":"Film.X.2160p","state":"downloading",
            "progress":0.81,"eta":540,"dlspeed":25000000}]})
        state=collect_media_state(jelly_client=jelly, radarr_client=radarr, qbit_client=qbit, now_unix=0)
        self.assertEqual(state["incoming"]["title"], "Film X")
        self.assertEqual(state["incoming"]["eta_minutes"], 9)
        self.assertEqual(state["incoming"]["speed_bytes_s"], 25000000)
        self.assertEqual(state["display"]["mode"], "incoming")


if __name__ == "__main__":
    unittest.main()


class AooScopeMediaSettingsTests(unittest.TestCase):
    def test_build_media_clients_from_settings_uses_saved_secrets(self):
        from aooscope.media import build_media_clients_from_settings
        settings = {"providers": {
            "jellyfin": {"enabled": True, "url": "http://media:8096", "verify_tls": True},
            "radarr": {"enabled": True, "url": "radarr:7878", "verify_tls": False},
            "qbittorrent": {"enabled": False, "url": "qbit:8080"},
        }}
        secrets = {"jellyfin": {"api_key": "j-key"}, "radarr": {"api_key": "r-key"}}
        clients = build_media_clients_from_settings(settings, secrets)
        self.assertEqual(clients["jellyfin"].headers["X-Emby-Token"], "j-key")
        self.assertEqual(clients["radarr"].headers["X-Api-Key"], "r-key")
        self.assertFalse(clients["radarr"].verify_tls)
        self.assertNotIn("qbittorrent", clients)
