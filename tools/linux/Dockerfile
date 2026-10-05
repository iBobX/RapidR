# A local Linux machine for RapidR's desktop host (no remote CI): the
# workspace built and its GUI tests run on Ubuntu, headless and through
# real X11 windows on a virtual display (Xvfb). See tools/linux/check.sh.
FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
# What the desktop host links on Linux: fontconfig (fontique), xkbcommon,
# X11 / Wayland (winit), ALSA (sounds); Xvfb and Mesa's software Vulkan
# (lavapipe) for real windows without a GPU; Node for the test drivers.
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl build-essential pkg-config git \
      libfontconfig1-dev libfreetype-dev libxkbcommon-dev libxkbcommon-x11-dev \
      libx11-dev libx11-xcb-dev libxcursor-dev libxrandr-dev libxi-dev libxcb1-dev \
      libwayland-dev libasound2-dev \
      xvfb xauth mesa-vulkan-drivers libvulkan1 \
      nodejs npm \
    && rm -rf /var/lib/apt/lists/*
ARG RUST=1.98.1
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain ${RUST}
ENV PATH=/root/.cargo/bin:$PATH
WORKDIR /src
