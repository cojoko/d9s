# d9s - Terminal UI for Dagster

## Overview

d9s is a [k9s](https://k9scli.io/)-inspired interface designed to help you monitor your [Dagster](https://dagster.io/) instances without ever leaving the comfort of your terminal. On-screen information is polled on tight, regular intervals to give a nearly real-time view of activity, and polling is dynamic to each view to avoid requesting irrelevant information. 

## Features

- Browse and monitor pipeline runs
- View pipeline details and configuration
- Search and filter pipelines and runs with fuzzy matching
- View run details including configuration and status
- Support for multiple Dagster instances via context switching
- Vim-inspired keybindings for efficient navigation

## Current Limitations

- Runs are limited to a context-specific number in order to avoid high network and memory costs associated with requesting potentially thousands of runs every few seconds. The Dagster API does support a cursor argument when fetching runs, but until the runs view is reworked to paginate its results, keep in mind run count will be limited. Please use a runs limit which works best for your use-case.
- d9s is currently read-only. You cannot launch runs, re-run failures, or terminate jobs from this tool. It is currently meant only for observation of one's Dagster deployments. 
- The feature-set for d9s is limited. I wrote this on paternity leave while the baby was napping, so please don't expect a full k9s suite of tools at your disposal. Right now you can search through and keep an eye on runs and pipelines. If this proves useful, I hope to add features like log viewing, asset support, graph visualizations, and color theming down the line. If you have feature requests, GitHub issues and pull requests are welcome!

## Keyboard Navigation

### Global
- `q` - Quit application
- `:` - Enter command mode
- `ESC` - Go back/cancel current action

### Command Mode
- `:runs` - View all runs
- `:pipelines` - View all pipelines
- `:contexts` - Manage connection contexts
- `:url <url>` - Set Dagster GraphQL URL
- `:context <name>` - Switch to a different context
- `:q` - Quit application

### List Views (Runs, Pipelines)
- `j` or `↓` - Move down
- `k` or `↑` - Move up
- `/` - Search/filter (with fuzzy matching)
- `Enter` - View details

### Detail View
- `j` or `↓` - Scroll down
- `k` or `↑` - Scroll up
- `h` or `←` - Scroll left
- `l` or `→` - Scroll right

### Context Management
- `a` - Add a new context
- `d` - Delete selected context
- `Enter` - Switch to selected context

## Configuration

Configuration is stored in `~/.config/d9s/config.toml` and includes:

```toml
last_context = "default"

[contexts.default]
url = "http://localhost:3000/graphql"
runs_limit = 20
```

You can manage contexts through the UI or directly edit this file.


