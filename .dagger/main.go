// .dagger/main.go
package main

import (
	"context"
	"srwm/internal/dagger"
)

type Srwm struct{}

func (m *Srwm) Build(ctx context.Context, source *dagger.Directory) *dagger.File {
	return dag.Container().
		From("rust:latest").
		WithEnvVariable("DEBIAN_FRONTEND", "noninteractive").
		WithExec([]string{"apt-get", "update"}).
		WithExec([]string{"apt-get", "install", "-y",
			// C compiler + pkg-config
			"gcc", "pkg-config",
			// X11 stack (needed by c-src/wm.h)
			"libx11-dev", "libxinerama-dev", "libxft-dev",
			"libxrender-dev", "libx11-xcb-dev",
			// Imlib2 (icon loading in drw.c)
			"libimlib2-dev",
			// Fontconfig + Freetype (font rendering)
			"libfontconfig1-dev", "libfreetype6-dev",
			// libclang (needed by bindgen to parse bridge.h)
			"libclang-dev",
		}).
		WithDirectory("/src", source.
			WithoutDirectory("target").
			WithoutDirectory("old-go-srwm"),
		).
		WithWorkdir("/src").
		WithExec([]string{"cargo", "build", "--release"}).
		File("target/release/srwm")
}
