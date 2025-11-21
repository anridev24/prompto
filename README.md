# prompto

**prompto** is a desktop application for codebase indexing and agentic prompt generation, powered by high-performance Rust libraries and an intuitive React UI (with shadcn/ui), all running inside a Tauri shell.

---

## Project Goals

- **Efficient Local Indexing:** Use Rust for fast codebase indexing and search.
- **Modern Desktop UI:** Ship as a true desktop app using [Tauri](https://tauri.app/) with a React (TypeScript) frontend. UI components styled using [shadcn/ui](https://ui.shadcn.com/).
- **Agentic Prompting:** Leverage the [Claude Agentic SDK](https://docs.anthropic.com/claude/docs/agentic) to generate and optimize prompts, utilizing local codebase knowledge.
- **Cross-Platform:** All major OSes supported (Windows, macOS, Linux) via Tauri.

---

## Getting Started

This project uses **Tauri** as the application framework.  
The backend (Rust) and frontend (React + TypeScript + shadcn/ui) are bundled together using Tauri.

> For full installation and usage details, always refer to the [official Tauri documentation](https://tauri.app/v1/guides/getting-started/prerequisites/) for your target OS.

### Prerequisites

- **Rust** (nightly toolchain recommended): [Install instructions](https://tauri.app/v1/guides/getting-started/prerequisites/#rust)
- **Node.js** (v18+): [Install instructions](https://nodejs.org/)
- **pnpm**, **yarn**, or **npm**: For frontend package management.
- Supported platforms: Windows, macOS, and Linux (see [Tauri system requirements](https://tauri.app/v1/guides/getting-started/prerequisites/)).

### Setup Workflow

1. **Clone this repository**

   ```sh
   git clone https://github.com/anridev24/prompto.git
   cd prompto
   ```

2. **Follow the official Tauri setup guide for your OS:**

   - [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites/)
   - [Tauri React Setup](https://tauri.app/v1/guides/getting-started/setup-react/)

3. **Structure (planned):**

   ```text
   /
   ├─ src-tauri/        # Rust backend (Tauri command handlers, integration, codebase indexing logic)
   └─ src/              # React frontend (TypeScript, shadcn/ui, UI, logic, prompt editor, etc)
   ```

4. **Development Mode**
   
   The standard Tauri React dev commands (see [official docs](https://tauri.app/v1/guides/getting-started/setup-react/#run-the-application)):
   
   ```sh
   # Install frontend dependencies
   npm install

   # (optional: install Rust dependencies)
   # cargo build

   # Start dev mode (frontend + backend)
   npm run tauri dev
   ```

5. **Build for Production**

   ```sh
   npm run tauri build
   ```

---

## Learn More

- **Tauri Docs:** [Getting Started](https://tauri.app/v1/guides/getting-started/)
- **shadcn/ui Docs:** [Installation](https://ui.shadcn.com/docs/installation)
- **Claude Agentic SDK:** [Docs](https://docs.anthropic.com/claude/docs/agentic)

---

> This project is in the design/bootstrapping stage. Core features, contributing guidelines, and implementation roadmap coming soon!
