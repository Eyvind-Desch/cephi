# Spotify einrichten

„Spiele Another Love von Tom Odell auf Spotify“ startet den Song in etwa einer Sekunde, ohne KI.

## So funktioniert es

1. Ein **Trigger** erkennt den Satz, das Modell wird nicht gefragt.
2. Orbit sucht den Song über die **Spotify Web API** (nur Suche, kein Login-Fenster nötig).
3. Orbit öffnet `spotify:track:<id>`, die **Spotify-App** spielt den Song sofort ab.

Pause, nächster Titel, lauter/leiser laufen über die **Medientasten** von Windows (`media_control`)
und brauchen keine API.

## Voraussetzungen

- Spotify-Desktop-App installiert und angemeldet
- **Spotify Premium**: Seit Februar 2026 verlangt Spotify Premium vom Besitzer jeder Entwickler-App
  ([Spotify: Migration Guide](https://developer.spotify.com/documentation/web-api/tutorials/february-2026-migration-guide))

## Einmalige Einrichtung (2 Minuten)

1. [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) öffnen und anmelden.
2. **Create app**
   - App name: `Orbit`, Description: beliebig
   - Redirect URI: `http://127.0.0.1:43821/callback` (Pflichtfeld, wird aktuell nicht benutzt)
   - API: **Web API** anhaken
   - Speichern
3. In der App → **Settings**: **Client ID** und **Client secret** (*View client secret*) kopieren.
4. In Orbit: großes Panel → **🔑 Schlüssel** → beide Werte einfügen → *Speichern*.

Fertig. Die Schlüssel liegen in der Windows-Anmeldeinformationsverwaltung (`Orbit/spotify_client_id`,
`Orbit/spotify_client_secret`), nicht in Dateien.

## Beispiele

| Du sagst | Ergebnis |
|---|---|
| `spiele another love von tom odell auf spotify` | Song läuft |
| `öffne mr brightside in spotify` | Song läuft |
| `spiele das album divide auf spotify` | Album wird geöffnet |
| `spiele die playlist lofi beats auf spotify` | Playlist wird geöffnet |
| `nächster song` / `pause` / `lauter` | Medientasten |
| `kannst du was von tom odell anmachen?` | geht an die KI, die `spotify_play` aufruft |

Bei Alben, Künstlern und Playlists öffnet Spotify die Seite. Ob die Wiedergabe dort automatisch startet,
entscheidet die Spotify-App. Bei einzelnen Songs startet sie zuverlässig.

## Eigene Sätze hinzufügen

In `%APPDATA%\Orbit\tools\media\spotify_play.json` unter `triggers` ergänzen, z. B.:

```json
"(mach|leg) {query} (auf|in) spotify an"
```
