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

Compatibility may depend on the specific distribution and the package-management configuration it provides.

Examples include:

* Arch Linux
* CachyOS
* EndeavourOS
* Manjaro
* Other Arch-based distributions

---

## Installation

### Arch User Repository

Repofy is intended to be distributed through the **Arch User Repository (AUR)**.

Once the AUR package is available, installation will be possible using an AUR helper such as:

```bash
yay -S repofy
```

or:

```bash
paru -S repofy
```

> The AUR package will be published once the project reaches its initial release.

### GitHub Releases

You can also download available releases directly from the project's GitHub repository:

https://github.com/GKPROEJECT/repofy/releases

---

## Usage

After installing Repofy, launch it from your application menu or from the terminal:

```bash
repofy
```

From the application you can browse available software and manage installations and removals through the graphical interface.

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
```

---

## Development

Clone the repository:

```bash
git clone https://github.com/GKPROEJECT/repofy.git
cd repofy
```

From there, follow the development/build instructions provided for the current release.

> Development instructions may change as Repofy evolves.

---

## Roadmap

Repofy is an actively evolving project.

Planned improvements include:

* [ ] Publish the first AUR package
* [ ] Improve software discovery
* [ ] Improve package information
* [ ] Improve installation feedback
* [ ] Improve uninstallation workflow
* [ ] Add more package-management features
* [ ] Improve the overall user experience
* [ ] Expand documentation

The roadmap may change as development progresses.

---

## Contributing

Contributions are welcome.

If you find a bug, have an idea or want to improve Repofy, you can open an issue or submit a pull request.

### Bug reports

When reporting a bug, please include:

* Your distribution
* Your desktop environment
* Repofy version
* Steps to reproduce the problem
* Relevant error messages or logs

### Feature requests

Feature requests are welcome as well. Please describe the problem you are trying to solve and how you think Repofy could improve the experience.

---

## License

Repofy is open-source software.

See the [`LICENSE`](LICENSE) file for the complete license information.

---

## Links

* **GitHub:** https://github.com/GKPROEJECT/repofy
* **Issues:** https://github.com/GKPROEJECT/repofy/issues
* **Releases:** https://github.com/GKPROEJECT/repofy/releases
* **Website:** https://github.com/GKPROEJECT/repofy-web

---

<p align="center">
  Made with ❤️ for the Linux community.
</p>
