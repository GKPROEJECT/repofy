# Repofy

<p align="center">
  <img src="logo.ico" alt="Repofy" width="96">
</p>

<h1 align="center">Repofy</h1>

<p align="center">
  A simple and modern software manager for Arch Linux and Arch-based distributions.
</p>

<p align="center">
  <a href="https://github.com/GKPROEJECT/repofy">GitHub</a>
  ·
  <a href="https://github.com/GKPROEJECT/repofy/issues">Issues</a>
  ·
  <a href="https://github.com/GKPROEJECT/repofy/releases">Releases</a>
</p>

---

## About

**Repofy** is a lightweight software management application designed for **Arch Linux and Arch-based distributions**.

Its goal is simple: make installing and removing software on Linux **easy, fast and accessible**, without requiring users to remember package-manager commands or navigate through complicated terminal workflows.

Repofy provides a graphical interface for discovering and managing software while keeping the underlying Arch package ecosystem intact.

> **Repofy — Software management, simplified.**

---

## Features

* 🖥️ **Graphical interface** — Manage software without relying on the terminal.
* 📦 **Easy installation** — Install applications with a few clicks.
* 🗑️ **Simple uninstallation** — Remove installed software just as easily.
* ⚡ **Fast and lightweight** — Designed to stay simple and responsive.
* 🐧 **Built for Arch** — Designed specifically around the Arch Linux ecosystem.
* 🔎 **Software discovery** — Find software without having to remember package names.
* 🎨 **Modern interface** — Clean and straightforward user experience.
* 🔧 **Native package management** — Works with the Arch package ecosystem instead of replacing it.

---

## Screenshot

<p align="center">
  <img src="captura.png" alt="Repofy screenshot" width="850">
</p>

---

## Supported distributions

Repofy is designed for **Arch Linux and Arch-based distributions**.

Compatibility may depend on the specific distribution and its package-management configuration.

Examples include:

* Arch Linux
* CachyOS
* EndeavourOS
* Manjaro
* Other Arch-based distributions

---

## Installation

### Current status

Repofy is currently in **early development** and is **not yet available as an installable package for end users**.

The current version is **0.1.1**.

The project is planned to be distributed through the **Arch User Repository (AUR)** once AUR distribution is available.

> **Note:** `yay -S repofy` and `paru -S repofy` will not work yet because the Repofy AUR package has not been published.

### GitHub Releases

The **v0.1.1** release is available on GitHub as the first public development release.

At this stage, the GitHub release contains the project source code and is not intended as a direct installation method for end users.

You can find the release here:

https://github.com/GKPROEJECT/repofy/releases

---

## Requirements

Repofy is designed for systems that use the **Arch Linux package ecosystem**.

### Runtime requirements

* Arch Linux or an Arch-based distribution
* A graphical desktop environment
* Access to the system package manager
* Internet connection for downloading software

---

## Project structure

The project is divided into the application itself and its accompanying resources.

The repository contains the source code, configuration and resources required to build and run Repofy.

```text
repofy/
├── src/
├── assets/
├── ...
├── LICENSE
└── README.md
