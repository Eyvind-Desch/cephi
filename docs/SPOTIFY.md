# Setting up Spotify

"Spiele Another Love von Tom Odell auf Spotify" ("Play Another Love by Tom Odell on Spotify") starts the song in about a second, without AI.

## How it works

1. A **trigger** recognizes the sentence; the model isn't asked.
2. Orbit looks up the song via the **Spotify Web API** (search only, no login window needed).
3. Orbit opens `spotify:track:<id>`, and the **Spotify app** plays the song immediately.

Pause, next track, and volume up/down run via Windows **media keys** (`media_control`)
and don't need any API.

## Requirements

- Spotify desktop app installed and signed in
- **Spotify Premium**: Since February 2026, Spotify requires Premium from the owner of every developer app
  ([Spotify: Migration Guide](https://developer.spotify.com/documentation/web-api/tutorials/february-2026-migration-guide))

## One-time setup (2 minutes)

1. Open [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) and sign in.
2. **Create app**
   - App name: `Orbit`, Description: anything
   - Redirect URI: `http://127.0.0.1:43821/callback` (required field, not currently used)
   - API: check **Web API**
   - Save
3. In the app → **Settings**: copy the **Client ID** and **Client secret** (*View client secret*).
4. In Orbit: large panel → **🔑 Keys** → paste both values → *Save*.

Done. The keys live in the Windows Credential Manager (`Orbit/spotify_client_id`,
`Orbit/spotify_client_secret`), not in files.

## Examples

| You say | Result |
|---|---|
| `spiele another love von tom odell auf spotify` / "play another love by tom odell on spotify" | Song plays |
| `öffne mr brightside in spotify` / "open mr brightside in spotify" | Song plays |
| `spiele das album divide auf spotify` / "play the album divide on spotify" | Album opens |
| `spiele die playlist lofi beats auf spotify` / "play the playlist lofi beats on spotify" | Playlist opens |
| `nächster song` / "next song", `pause` / "pause", `lauter` / "louder" | Media keys |
| `kannst du was von tom odell anmachen?` / "can you put on something by tom odell?" | Goes to the AI, which calls `spotify_play` |

For albums, artists, and playlists, Spotify opens the page. Whether playback starts automatically there
is up to the Spotify app. For individual songs, it starts reliably.

## Adding your own phrases

Add to `triggers` in `%APPDATA%\Orbit\tools\media\spotify_play.json`, e.g.:

```json
"(mach|leg) {query} (auf|in) spotify an"
```
