A bot to help the board with their meeting agenda and meeting
reminders.

## Requirements

The binary itself depends on:

- OpenSSL

... as well as the usual suspects:

```
$ ldd target/debug/agenda-bot
        linux-vdso.so.1 (0x00007ffc353fd000)
        libssl.so.1.1 => /usr/lib/libssl.so.1.1 (0x00007f58987d2000)
        libcrypto.so.1.1 => /usr/lib/libcrypto.so.1.1 (0x00007f58984f4000)
        libdl.so.2 => /usr/lib/libdl.so.2 (0x00007f58984ee000)
        libpthread.so.0 => /usr/lib/libpthread.so.0 (0x00007f58984cc000)
        libgcc_s.so.1 => /usr/lib/libgcc_s.so.1 (0x00007f58984b2000)
        libc.so.6 => /usr/lib/libc.so.6 (0x00007f58982e9000)
        /lib64/ld-linux-x86-64.so.2 => /usr/lib64/ld-linux-x86-64.so.2 (0x00007f5899b1b000)
        libm.so.6 => /usr/lib/libm.so.6 (0x00007f58981a1000)
```

It has only been tested on Linux.

Rust stable is needed to compile.

## Building

In order to actually use the bot you need:

- Somewhere for it to live
- A Slack "classic" bot user
- A Discord bot user
- Permission to add bots to your Slack workspace and Discord server

Then, either pass the bot tokens as enviornment variables (`DISCORD_API_TOKEN` and
`SLACK_API_TOKEN`), or hard-code them into the binary (**NOT RECOMMENDED**
except for development purposes) by editing `src/discord.rs` and `src/slack.rs`.

Which channels the messages are sent to is currently specified via either
hard-coded constant values or environment variables (`DISCORD_CHANNEL` and
`SLACK_CHANNEL`).

The following shows all necessary steps needed to build and run the bot:

```shell
$ git clone https://github.com/lithekod/agenda-bot.git
$ cd agenda-bot
$ DISCORD_API_TOKEN=""     \ # fill
        SLACK_API_TOKEN="" \ # in
        DISCORD_CHANNEL="" \ # your
        SLACK_CHANNEL=""   \ # values
        cargo run
```

## Current (non-)features

- Messages are sent where they should
- ...but they aren't stored anywhere and can't be summarized.
- No reminders.
- No permissions / trusted users / trusted channels. Please, only private
  testing servers for now.

See the TODO for more planned features.

## Sales pitch (not yet implemented)

Board members can add items to the agenda by sending a message
containing something like

```
/agenda Kaffet är slut!
```

in either Slack or Discord. The bot sends a confirmation in both Slack
and Discord so everyone can see what's being added.

Every Wednesday afternoon (configurable) the bot sends a reminder and the agenda
in both Slack and Discord. An additional reminder is sent 1 hour before the
meeting.
