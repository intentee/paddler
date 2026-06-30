#!/usr/bin/env bash
set -euo pipefail

ghostty_window_count() {
    swaymsg -t get_tree | jq '[.. | objects | select(.app_id == "com.mitchellh.ghostty")] | length'
}

open_session() {
    local before current
    before="$(ghostty_window_count)"
    setsid -f ghostty --working-directory="$1" -e claude --continue
    current="$before"
    while [ "$current" -le "$before" ]; do
        current="$(ghostty_window_count)"
    done
}

open_session '/home/mcharytoniuk/workspace/intentee/coder'
open_session '/home/mcharytoniuk/workspace/intentee/intentee'
open_session '/home/mcharytoniuk/workspace/intentee/paddler'
open_session '/home/mcharytoniuk/workspace/intentee/poet'
