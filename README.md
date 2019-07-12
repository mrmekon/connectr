# ![icon](connectr_80px_300dpi.png) Connectr ![icon](connectr_80px_300dpi.png)
[![OSX/Linux Build Status](https://travis-ci.org/mrmekon/connectr.svg?branch=master)](https://travis-ci.org/mrmekon/connectr)
[![Windows Build Status](https://ci.appveyor.com/api/projects/status/4afwy0yj2477f84h/branch/master?svg=true)](https://ci.appveyor.com/project/mrmekon/connectr/branch/master)
[![Crates.io Version](https://img.shields.io/crates/v/connectr.svg)](https://crates.io/crates/connectr)

##
### A super lightweight Spotify controller.
##

Connectr is a tiny application that lets you quickly and easily see – or change – what's playing on your Spotify account.

It's compatible with anything that supports [Spotify Connect](https://www.spotify.com/se/connect/): computers, mobiles, speakers, TVs, Playstations, etc.

***NOTE:*** Spotify Premium is required to use Spotify's remote control features.  Free accounts do not work.

It runs in the Mac menu bar (top right corner) or Windows system tray (bottom right corner...), and the Mac Touch Bar (the fancy touchscreen on new Macbook keyboards).  It's just a little icon that pops up the controls when you need them, and hides away when you don't.  Or just hover over it to see what's currently playing.

What it can do:

* Show what's playing
* Play/pause
* Skip tracks
* Quick-play a saved 'preset'
* Quick-save playing track to a playlist
* Select playback device
* Change volume
* Alarm clock (play on a selected device at a specific time)
* Scrobble to Last.fm

Most importantly, it maintains a tiny memory footprint while running.  ~10MB on a Mac, compared to 300-1000MB for the Spotify desktop app.  You shouldn't need to buy extra RAM just to monitor what's playing on your speakers.

The alarm clock and scrobbling features expect Connectr to run on an always-on server.  If you want to run it on a headless Linux machine, you can configure it on a local machine first and then move the `~/.connectr.ini` to your server.

For developers: the API for communicating with the Spotify backend is provided as a Rust library, available as a Cargo crate. Connectr exposes the official [Spotify 'Player' Web API](https://developer.spotify.com/web-api/web-api-connect-endpoint-reference/) for controlling Spotify Connect devices.

***NOTE:*** Connectr is not an audio playback tool; it's just a remote control.  Spotify has not publicly released a library for implementing audio playback with Spotify Connect support.  There's a reverse engineering effort, coincidentally also in Rust, at [librespot](https://github.com/plietar/librespot).  The librespot + connectr combo gives you a full Spotify playback experience in ~15MB of RAM.  It's the most resource-efficient way to listen to Spotify.

***NOTE:*** Connectr is not developed by Spotify and has nothing to do with their company.

## Download

*Binary releases are provided as a convenience.  Building from source is preferred.*

### Connectr v1.1.0
#### [Mac OS X](https://github.com/mrmekon/connectr/releases/download/connectr-1.1.0/connectr-1.1.0.zip) (64-bit, md5: ce958529bcf3fef1ece3c73b1197678d)

### OLD RELEASE: Connectr v0.2.0
#### [Windows 10](https://github.com/mrmekon/connectr/releases/download/connectr-0.2.0/connectr-0.2.0-win32.zip) (64-bit, md5: 1745ecb67bd5ef0822eeabd96d98dcde)

## Screenshots

##

<table><tr><td valign="top">

<center><strong>Mac OS X</strong></center></br>
<center><img src="https://github.com/mrmekon/connectr/blob/master/docs/screenshot.png?raw=true" width="300"></center>

</td><td valign="top">

<center><strong>Windows 10</strong></center></br>
<center><img src="https://github.com/mrmekon/connectr/blob/master/docs/screenshot_windows.png?raw=true" width="300"></center>

</td></tr><tr><td colspan="2">

<center><strong>Macbook Touch Bar</strong></center></br>
<center><img src="https://github.com/mrmekon/connectr/blob/master/docs/screenshot_touchbar.png?raw=true"></center>

</td></tr></table>

##


## Build Instructions

You need to [download Rust](https://www.rustup.rs/) to build.

```
$ git clone https://github.com/mrmekon/connectr.git
$ cd connectr
$ cargo run
```

## Usage / Help

On first launch, Connectr will open your web browser to a self-configuration page, and save its configuration to your system home directory.  The configuration page will walk you through creating the necessary Spotify developer application.

### Spotify Developer Application configuration

On the first launch, Connectr will guide you through setting up a Spotify developer application.  If you want to do it manually instead, or if something goes wrong, here are the instructions:

* Go to your [Spotify Applications](https://developer.spotify.com/my-applications/#!/applications/create) page (login with your Spotify credentials)
* Click "CREATE AN APP" in the upper-right corner
* Enter a name (perhaps "Connectr") and description ("Use Connectr app with my account.")
* Add a Redirect URI: <em>http://127.0.0.1:5432</em>
* Copy your <em>Client ID</em> and <em>Client Secret</em> to `connectr.ini` (see below).

### Mac Touch Bar interface

#### Setup
* Be sure the "Control Strip" is enabled in the Keyboard section of System Preferences, under `Touch Bar shows`.
* Press the Connectr icon in the Control Strip to expand it.  It will stay expanded until you press the `x` button on the left side.
* The Control Strip only supports 4 icons, and stacks all new ones on the left-most icon.  If the Connectr icon is missing, it may be hidden "under" another icon.  You can keep clicking the left-most icon to open them all.

#### Controls
* Double-tap the track title to rotate through modes:
 * Track and Artist
 * Track
 * Artist
* To quick-save a track, swipe right on track title until a box is drawn around it and release.  Configure quick-save in `connectr.ini` first.

### Configuration file (connectr.ini) format

**Note:** connectr uses `~/.connectr.ini` if it exists.  If it does _not_ exist, connectr will fallback to trying `connectr.ini` in the directory it is run from.  A template is provided in `connectr.ini.in`.

The config file is generated by a graphical web configuration the first time Connectr is launched, and can be reconfigured by selecting `Reconfigure Connectr` from the menu.  It is not necessary to write `connectr.ini` yourself.  The following documentation is just for reference.

connectr's configuration is read from a regular INI file with these sections:

#### [connectr]
* **port** - Port to temporarily run web server on when requesting initial OAuth tokens (integer).  Default is 5432. _ex: `port = 5432`_
* **quicksave_default** - Playlist to save tracks to when 'Quick-Save' is selected

#### [application]
* **client_id** - Spotify web application's Client ID (string). _ex: `client_id = ABCABCABCABC123123123`_
* **secret** - Spotify web application's Client Secret (string). _ex: `secret = DEFDEFDEFDEF456456456`_

#### [presets]

One preset per line, in either format:

* [Preset Name] = [Context URI]
* [Preset Name] = [Context URI],[Quick-save Playlist URI]

Where:

* `Preset Name` is any name you want for the preset
* `Context URI` is the Spotify context (album, playlist, etc) to play when selected
* `Quick-save URI` is (optionally) a playlist to save the current track to if 'Quick-Save' is clicked while this preset is playing.

*Example:*

Make a preset called `Bakesale` that plays a Sebadoh album when selected, and saves my favorite tracks from that album to a private playlist:

`Bakesale = spotify:album:70XjdLKH7HHsFVWoQipP0T,spotify:user:mrmekon:playlist:4aqg0RkXSxknWvIXARV7or`

#### [alarms]
_Note: This can and should be configured through the graphical web interface instead of by editing directly.  Select `Edit Alarms` from the Connectr menu to launch the graphical interface._

Up to five alarm clock entries, which specify a time, device, playlist to play, and which days to repeat the alarm on.

Format:

`alarm<i> = <hour>:<minute>,<repeat>,<Spotify URI>,<Device ID>`

* **`<i>`** - Number between 0 and 4 (inclusive)
* **`<hour>`** - Hour in 24-hour time (0-23)
* **`<minute>`** - Minute (0-59)
* **`<repeat>`** - One of: `daily`, `weekdays`, `weekends`
* **`<Spotify URI>`** - URI of a Spotify context to play.  Same format as presets.
* **`<Device ID>`** - Unique ID of the device to play on.  These are listed on the graphical web interface, or can be found in the `~/.connectr.log` log file.

_note: Connectr must be running and connected to the internet at the scheduled alarm time.  The target device must also be running and logged in with your Spotify account.  This means the alarm functionality is most useful when running on an always-on machine such as a home media server or a VPS.  You can run Connectr on a headless server by configuring it on a desktop machine, and copying the `~/.connectr.ini` config to the server._

#### [lastfm]

Optional configuration to have Connectr scrobble track plays to the Last.fm scrobbling service.  This requires a free Last.fm account and free Last.fm developer API tokens.

_note: This MUST be configured through the graphical web interface.  The web interface requests your Last.fm username and password, and the password is swapped out for a session key before saving to the config file.  It is not possible to specify a password in the config file, so you cannot enable Last.fm scrobbling without the GUI.  Once enabled, a valid Last.fm configuration can be transferred to other machines._

_note: Like the alarm feature, scrobbling requires Connectr to always be running.  This means it should be run from an always-on computer, such as a home media server or a VPS.  You can configure it on a regular machine first, and then copy the ¨`/.connectr.ini` file to your always-on server._

There are options to ignore tracks played on phones/tablets or computers, in case you want to have the official Spotify clients handle scrobbling from those devices.  This is beneficial, especially for mobiles, because Spotify can scrobble tracks played while offline.

* **enabled** - Whether Last.fm scrobbling is enabled
* **key** - Last.fm developer API key
* **secret** - Last.fm developer API secret
* **session_key** - Cached Last.fm authentication token
* **username** - Last.fm username
* **ignore_pc** - Whether Connectr should ignore tracks played on a computer
* **ignore_phone** - Whether Connectr should ignore tracks played on a phone/tablet

#### [tokens]
_note: This section is auto-generated and auto-updated at runtime.  You can leave it empty if you make your own config file._

* **version** - Version of the Connectr authentication format
* **access** - Spotify Web API access token
* **refresh** - Spotify Web API refresh token
* **expire** - Expiration time (UTC) for access token


#### Example connectr.ini
```
[connectr]
port=5432

[application]
secret=xxxxxyyyyyaaaaabbbbbcccccddddd
client_id=xXxXxyYyYynNnNnNmMmMmMpPpPpP

[presets]
Discover Weekly=spotify:user:spotify:playlist:37i9dQZEVXcOmDhsenkuCu
Edge Detector=spotify:user:mrmekon:playlist:4SKkpDbZwNGklpIILmEZAg
Play Today=spotify:user:mrmekon:playlist:4c8eKK6kKrcdt1HToEX7Jc

[tokens]
version=1
access=this-is-autogenerated
refresh=this-is-also-autogenerated
expire=1492766270

[lastfm]
enabled=true
key=aaaaabbbbbbccccccddddddeeeeee
secret=ffffffgggggghhhhhhhiiiiiijjjjjj
session_key=kkkkkkllllllmmmmmmnnnnnooooooppppp
username=MyGloriousUsername
ignore_phone=true
ignore_pc=false

[alarms]
alarm1=08:00,weekdays,spotify:user:mrmekon:playlist:1BayoBGuBA5HhF0ZuYw2sN,1267eba791c19740744eb5c41a5165ce6691fb9b
```

### Feature Progress

| Feature                                | OS X                    | Windows                 | Linux                   |
| ---                                    | ---                     | ---                     | ---                     |
|                                        |
|                                        |
| **API**                                |
| Fetch list of devices                  | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Fetch current playback information     | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Transfer playback to device            | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Start new playback on device           | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Pause/Resume                           | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Skip next/previous                     | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Seek in track                          | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Change volume                          | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Change repeat state                    | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Change shuffle state                   | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Alarm clock                            | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Last.fm Scrobbling                     | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| Fetch album art                        | <ul><li> [ ] </li></ul> | <ul><li> [ ] </li></ul> | <ul><li> [ ] </li></ul> |
|                                        |
|                                        |
| **UI**                                 |
| Display current track                  | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [ ] </li></ul> |
| Current track in tooltip               | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [ ] </li></ul> |
| Playback controls                      | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [ ] </li></ul> |
| Device selection                       | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [ ] </li></ul> |
| Volume control                         | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [ ] </li></ul> |
| Presets                                | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [ ] </li></ul> |
| Save current track to playlist         | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [ ] </li></ul> |
|                                        |
|                                        |
| **System**                             |
| Persistent configuration               | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |
| System logging                         | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> | <ul><li> [x] </li></ul> |


## Notable Dependencies

* [fruitbasket](https://github.com/mrmekon/fruitbasket) - Framework for Mac app lifecycle, written for Connectr.
* [rubrail](https://github.com/mrmekon/rubrail-rs) - Mac Touch Bar library, written for Connectr.
* [objc](https://github.com/SSheldon/rust-objc/) - SSheldon's suite of Objective-C wrappers for Rust.
* [cocoa-rs](https://github.com/servo/cocoa-rs) - Cocoa bindings for Rust, which complement `objc`.
* [systray](https://github.com/qdot/systray-rs) - Windows systray library for Rust.
* [rustfm-scrobble](https://github.com/bobbo/rustfm-scrobble) - Last.fm scrobbling library.
