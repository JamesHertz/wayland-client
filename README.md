# Wayland-Client

Have you ever wondered how does windows appear on the screen? Well, I did so I went searching and discovered interesting things, such as the legacy inheritance named [X11](https://en.wikipedia.org/wiki/X_Window_System) and the only implementation of the specification named [X-server](https://www.x.org/releases/X11R7.6/doc/man/man1/Xserver.1.xhtml). I also learned about the new hot thing [Wayland](https://en.wikipedia.org/wiki/Wayland_(protocol)) and [why it is better than X11](https://wayland.freedesktop.org/docs/html/ch01.html).

In the end, I wanted to make a Wayland compositor, but it seemed to be quite a challenge. So I decided to do the obvious which is taking baby steps. And here I am building a Wayland client from scratch with no third-party library except, Linux syscalls not available at the Rust standard library and a logging library.

Hope you find it useful because I am having a good time.
