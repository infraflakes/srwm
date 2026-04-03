#!/bin/sh
# Window search: list open windows via foreign-toplevel, pick with fuzzel, activate.
# Requires: wlrctl, fuzzel

XDG_DATA_DIRS="${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"

lookup_desktop() {
    id="$1"
    # Pass 1: match by filename (app_id.desktop or *app_id*.desktop)
    for dir in "$HOME/.local/share/applications" $(printf '%s' "$XDG_DATA_DIRS" | tr ':' ' '); do
        for f in "$dir/$id.desktop" "$dir"/*"$id"*.desktop; do
            [ -f "$f" ] || continue
            name=$(grep -m1 '^Name=' "$f" | cut -d= -f2-)
            icon=$(grep -m1 '^Icon=' "$f" | cut -d= -f2-)
            [ -n "$name" ] && printf '%s\t%s' "$name" "${icon:-$id}" && return
        done
    done
    # Pass 2: match by StartupWMClass (e.g. REAPER -> cockos-reaper.desktop)
    for dir in "$HOME/.local/share/applications" $(printf '%s' "$XDG_DATA_DIRS" | tr ':' ' '); do
        [ -d "$dir" ] || continue
        f=$(grep -rl "^StartupWMClass=$id$" "$dir"/*.desktop 2>/dev/null | head -1)
        if [ -n "$f" ]; then
            name=$(grep -m1 '^Name=' "$f" | cut -d= -f2-)
            icon=$(grep -m1 '^Icon=' "$f" | cut -d= -f2-)
            [ -n "$name" ] && printf '%s\t%s' "$name" "${icon:-$id}" && return
        fi
    done
    printf '%s\t%s' "$id" "$id"
}

# Build two files: one for fuzzel display, one as lookup table
display=$(mktemp)
lookup=$(mktemp)
trap 'rm -f "$display" "$lookup"' EXIT

i=0
wlrctl toplevel list | while IFS= read -r line; do
    app_id="${line%%: *}"
    title="${line#*: }"
    desktop=$(lookup_desktop "$app_id")
    app_name="${desktop%%	*}"
    icon="${desktop#*	}"
    # Display file: "title  app_name" with icon (app_name searchable but visually secondary)
    printf '%s  %s\0icon\x1f%s\n' "$title" "$app_name" "$icon" >> "$display"
    # Lookup file: line-indexed app_id and title for focus
    printf '%s\t%s\n' "$app_id" "$title" >> "$lookup"
    i=$((i + 1))
done

[ -s "$display" ] || exit 0

selected=$(fuzzel --dmenu \
    --prompt="Window: " \
    --no-run-if-empty \
    --index \
    < "$display")

[ -z "$selected" ] && exit 0

# --index gives the 0-based line number
line_num=$((selected + 1))
match=$(sed -n "${line_num}p" "$lookup")
sel_app_id="$(printf '%s' "$match" | cut -f1)"
sel_title="$(printf '%s' "$match" | cut -f2)"

exec wlrctl toplevel focus "app_id:$sel_app_id" "title:$sel_title"
