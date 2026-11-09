import Link from "next/link";
import { Metadata } from "next";

export const metadata: Metadata = {
  title: "Stellar Multisig API — Docs",
  description: "Documentation for the Stellar Multisig Coordinator API",
};

const NAV = [
  { label: "Getting Started", href: "/docs/getting-started" },
  { label: "Classic Multisig", href: "/docs/concepts/classic-multisig" },
  { label: "Soroban Smart Accounts", href: "/docs/concepts/soroban-smart-accounts" },
  { label: "Security Model", href: "/docs/concepts/security-model" },
  { label: "Multisig Account", href: "/docs/contracts/multisig-account" },
  { label: "Account Factory", href: "/docs/contracts/account-factory" },
  { label: "API Overview", href: "/docs/api/overview" },
  { label: "Proposals", href: "/docs/api/proposals" },
  { label: "Signatures", href: "/docs/api/signatures" },
  { label: "Accounts", href: "/docs/api/accounts" },
  { label: "Web App Guide", href: "/docs/web-app-guide" },
  { label: "Integration", href: "/docs/integration" },
];

export default function DocsLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased min-h-screen bg-[#0f172a] text-[#f8fafc]">
        <nav className="border-b border-[#334155] bg-[#1e293b]/50 backdrop-blur sticky top-0 z-50">
          <div className="max-w-6xl mx-auto px-4">
            <Link
              href="/docs/getting-started"
              className="flex items-center gap-2 px-4 py-3 font-semibold text-[#f8fafc]"
            >
              <span className="text-[#38bdf8]">★</span> Stellar Multisig
            </Link>
            <div className="flex gap-1 overflow-x-auto pb-2 scrollbar-hide">
              {NAV.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="whitespace-nowrap px-3 py-2 text-sm text-[#94a3b8] hover:text-[#f8fafc] rounded-md hover:bg-[#334155] transition-colors"
                >
                  {item.label}
                </Link>
              ))}
            </div>
          </div>
        </nav>

        <main className="max-w-3xl mx-auto px-4 py-8">
          {children}
        </main>

        <footer className="border-t border-[#334155] bg-[#1e293b] py-8 mt-12">
          <div className="max-w-6xl mx-auto px-4 text-[#64748b] text-sm">
            <p>
              <strong>Stellar Multisig API + Docs</strong> — Coordinator API for
              Stellar multisig (M-of-N) proposal signing.
            </p>
            <p className="mt-2">
              Sister repos:
              {" "}
              <a
                href="https://github.com/stellar-multisig/stellar-multisig-contracts"
                className="text-[#38bdf8] hover:underline"
              >
                Contracts
              </a>
              {", "}
              <a
                href="https://github.com/stellar-multisig/stellar-multisig-web"
                className="text-[#38bdf8] hover:underline"
              >
                Web App
              </a>
            </p>
          </div>
        </footer>
      </body>
    </html>
  );
}
