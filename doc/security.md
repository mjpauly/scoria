# Scoria Security

## Scope

Scoria is software the runs locally on the user's device. It aims to be prevent
other low-privilege apps running on the user's device from gaining access to
sensitive data in Scoria.

The OS's app sandboxing plays a significant role in Scoria's security. The
app's filesystem, where the user's data is stored, is isolated from other apps
installed on the device.

## Tooling

Scoria's plans involve distributing apps on multiple different platforms and
having substantial serverside and web components. In order to minimize
complexity, a set of tools which are maximally reusable across all software
components is desired. Ideally, the same set of tools can be used for
developing the multiplatform app, API backends, and app/web frontends. This
minimizes the range of tooling the developers need to be familiar with.

Rust was settled upon as a language which met this goal. It can be compiled for
the major mobile platforms, for the web browser as WebAssembly, and for server
and API backends. Many of Rust's open source libraries such as actix-web, sqlx,
tokio, serde, and yew are mature and well tested. Furthermore, Rust's emphasis
on safety means the compiler does an large amount of work to check code for
pitfalls common in other languages, further speeding up development.

## App Architecture

Scoria is designed such that nearly all core functionality is implemented in
Rust, including the app's user interface. System calls made to the OS and its
frameworks are mainly written in Swift on iOS (and likely Java for Android, when
that dev happens). We'll detail the iOS architecture from here on.

A Swift binary responsible for launching the app and handling OS communication
is statically linked to the Rust binary via a C foreign function interface. At
startup, the Rust code starts its own runtime in a separate thread.

Whenever the app is foregrounded, it starts up an `actix-web` http
server, which serves a web-based frontend for the user to interact with. The
frontend is loaded by Swift in a `WkWebView`, a framework provided by iOS to
integrate web content into apps. The backend server listens on `127.0.0.1`, the
device's loopback address using a random open port. It also generates a 64-bit
secret with `rand::rngs::ThreadRng`, which uses a cryptographically secure PRNG
seeded with system entropy.

All UI resources served from the http server are scoped behind the secret. For 
example, if the secret is `78161` and the port is `54039`, the index is only
accessible at `127.0.0.1:54039/78161/`. Attempts to access resources at any
other path are rejected by the backend server. This secret is passed to Swift
via the C FFI, and it loads the frontend at the index, which then pulls in other
static assets at the same scope, including the WebAssembly binary which runs in
the WkWebView. Because it listems on the device's loopback address, any other
low-privilege user program can attempt to connect.

When it initializes, the frontend code connects to the backend server via a
websocket connection, the path to which is also hidden behind the shared secret.
This websocket connection is the channel through which the UI and the app
backend share state and location data while the app is foregrounded.

Since sensitive data is accessible through this connection, it's important that
other user programs cannot access it. Once the connection is established, the
backend refuses any further websocket connections. Upon app backgrounding, the
server shuts down.

There is only a small window of time when the app is foregrounded that
there is a possibility of the websocket connection getting hijacked by another
user program (that is, if it manages to correctly guess the 64-bit secret). This
window of time is between when the app foregrounding triggers the server startup
and when the WebAssembly loaded in the WkWebView reaches the point in its
initialization when it connects to the websocket. This takes at most a few
seconds. Even if the server responded to an absurd one million requests
per second from a brute force attack, after ten seconds (and ten million
guesses) the probability of a correct guess is less than one ten-billionth of
one percent.

Since the WkWebView is sandboxed from other apps, it's not possible for other
apps to see what paths (and hence the secret) Scoria's frontend is loading
resources from.

We believe this provides sufficient security to protect the one part of the app
that is at risk of exposure to other apps running on the user's device.
