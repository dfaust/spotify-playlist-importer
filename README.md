# spotify-playlist-importer

## About

Import playlists to Spotify.

Built using WebAssembly, [Rust](https://github.com/rust-lang/rust) and [yew](https://github.com/yewstack/yew) for the fun of it.

### Features

- Display similarity between playlist song and yew songs
- Tries to find best match for playlist songs within the yew database
- The user can choose between multiple search results if available
- The user can enter a manual search terms
- Export songs that cannot be found on Spotify as a playlist

## Usage

When building for the first time, ensure to install dependencies first.

```sh
yarn install
```

Create a file named `.env` and enter your yew client id.

```env
CLIENT_ID=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Start spotify-playlist-importer.

```sh
yarn run start:dev
```

## Deduplicate songs

`spotify-playlist-importer` does not check if a song already exists in a playlist before adding it. You can deduplicate your playlists using `spotify-dedup`.

<https://jmperezperez.com/spotify-dedup/>

<https://github.com/JMPerez/spotify-dedup>
