# deltav DevSecOps Dashboard Development Container
FROM ubuntu:24.04

# Avoid prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install base dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    curl \
    wget \
    ca-certificates \
    gnupg \
    pkg-config \
    make \
    sudo \
    locales \
    && rm -rf /var/lib/apt/lists/*

# Set up locale
RUN locale-gen en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# Install Rust via rustup
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="${CARGO_HOME}/bin:${PATH}"

# Add WASM target for Leptos frontend
RUN rustup target add wasm32-unknown-unknown

# Install Rust development tools
RUN cargo install cargo-deny cargo-audit cargo-leptos cargo-watch

# Install Claude Code CLI (standalone installer, no Node.js required)
RUN curl -fsSL https://claude.ai/install.sh | bash

# Create non-root user for development (handle existing UID/GID 1000)
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=${USER_UID}

RUN if getent group ${USER_GID} > /dev/null 2>&1; then \
    existing_group=$(getent group ${USER_GID} | cut -d: -f1); \
    groupmod -n ${USERNAME} ${existing_group}; \
    else \
    groupadd --gid ${USER_GID} ${USERNAME}; \
    fi \
    && if getent passwd ${USER_UID} > /dev/null 2>&1; then \
    existing_user=$(getent passwd ${USER_UID} | cut -d: -f1); \
    usermod -l ${USERNAME} -d /home/${USERNAME} -m ${existing_user}; \
    else \
    useradd --uid ${USER_UID} --gid ${USER_GID} -m ${USERNAME}; \
    fi \
    && echo "${USERNAME} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/${USERNAME} \
    && chmod 0440 /etc/sudoers.d/${USERNAME}

# Create .claude directory for user
RUN mkdir -p /home/${USERNAME}/.claude \
    && chown -R ${USERNAME}:${USERNAME} /home/${USERNAME}/.claude

# Add Rust/Cargo to path in bashrc
# Use user-specific CARGO_HOME so vscode user can download crates and install tools
RUN echo '# Rust environment' >> /home/${USERNAME}/.bashrc \
    && echo 'export CARGO_HOME="${HOME}/.cargo"' >> /home/${USERNAME}/.bashrc \
    && echo 'export PATH="/opt/cargo/bin:${HOME}/.cargo/bin:${PATH}"' >> /home/${USERNAME}/.bashrc

# Final user switch for workdir
USER ${USERNAME}
WORKDIR /workspaces
