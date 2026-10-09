# Audio fixtures

Both files hold one second of a 440 Hz sine tone, mono, 44100 Hz. They were generated for
these tests with FFmpeg 8.1.3 and contain nothing else:

```sh
ffmpeg -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=1" -ac 1 \
    -c:a libvorbis -q:a 2 -map_metadata -1 -fflags +bitexact -flags:a +bitexact tone.ogg
ffmpeg -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=1" -ac 1 \
    -c:a libmp3lame -b:a 64k -map_metadata -1 -fflags +bitexact -flags:a +bitexact tone.mp3
```
