package main

import (
	"context"
	"srwm-wayland/internal/dagger"
)

type Srwm struct{}

func (m *Srcom) Build(ctx context.Context, source *dagger.Directory) *dagger.File {
	return dag.Container().
		From("rust:latest").
		WithEnvVariable("DEBIAN_FRONTEND", "noninteractive").
		WithExec([]string{"apt-get", "update"}).
		WithExec([]string{"apt-get", "install", "-y",
			"libconfig-dev", "uthash-dev", "libx11-xcb-dev",
			"libxcb-util-dev", "cmake", "git",
			"pkg-config", "libseat-dev", "libdisplay-info-dev",
			"libinput-dev", "libudev-dev", "libgbm-dev",
			"libxkbcommon-dev", "libwayland-dev", "libdrm-dev",
			"libpixman-1-dev", "libx11-dev", "libxcursor-dev",
			"libxrandr-dev", "libxi-dev", "libxcb1-dev",
			"libgl-dev",
		}).
		WithDirectory("/src", source.WithoutDirectory("build")).
		WithWorkdir("/src").
		WithExec([]string{"cargo", "build", "--release"}).
		File("target/release/srwm")
}
