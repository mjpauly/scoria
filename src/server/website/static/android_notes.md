# Scoria for Android


<div class="flex my-6 justify-center">
    <div class="flex flex-col items-center">
        <span class="text-lg font-medium text-neutral-200">
            Scoria 1.4.2
        </span>
        <a 
            class="underline font-light text-sm mb-6 text-neutral-200"
            href="/android/release-notes"
        >
            see changes
        </a>
        <a 
            href="/download/apk/latest"
            class="rounded-lg bg-primary px-6 py-3 text-xl w-min whitespace-nowrap"
        >
            Download APK
        </a>
        <span class="text-sm text-neutral-400 mt-2">
            Works on Andriod 12+
        </span>
    </div>
</div>

Scoria for Android is available only from [scoria.info](https://scoria.info).

You will need to [opt-in for installing unknown
apps](https://developer.android.com/distribute/marketing-tools/alternative-distribution#unknown-sources).

Scoria automatically checks for updates at startup, and will notify you in the
app if there is one available.

<hr class="mt-8 border-neutral-600">

## Verify App Signature

You can verify the signing certificate on the APK matches these fingerprints:

```
SHA-256: 3b58a8432237b982eaf18dc00285bdcf31d2338669678aaa68c51ec7f9b64845
SHA-1: dc5d5c59459c41861a377fb709ff8ed353b7b667
```

With [VirusTotal](https://www.virustotal.com/gui/home/upload), a virus
scanning website owned by Google:

```
Thumbprint dc5d5c59459c41861a377fb709ff8ed353b7b667
```

With [apksigner](https://developer.android.com/tools/apksigner), part of the
Android SDK:

```
$ apksigner verify -v --print-certs Scoria_1.3.0.apk
...
Signer #1 certificate SHA-256 digest: 3b58a8432237b982eaf18dc00285bdcf31d2338669678aaa68c51ec7f9b64845
Signer #1 certificate SHA-1 digest: dc5d5c59459c41861a377fb709ff8ed353b7b667
...
```

With [keytool](https://docs.oracle.com/en/java/javase/11/tools/keytool.html),
part of the standard Java distribution:

```
$ keytool -printcert -jarfile Scoria_1.3.0.apk
...
Certificate fingerprints:
	 SHA1: DC:5D:5C:59:45:9C:41:86:1A:37:7F:B7:09:FF:8E:D3:53:B7:B6:67
	 SHA256: 3B:58:A8:43:22:37:B9:82:EA:F1:8D:C0:02:85:BD:CF:31:D2:33:86:69:67:8A:AA:68:C5:1E:C7:F9:B6:48:45
...
```
