<template>
  <div class="app-shell">
    <NuxtRouteAnnouncer />
    <NuxtPage />
  </div>
</template>

<style>
  * {
    box-sizing: border-box;
  }

  body {
    margin: 0;
    font-family:
      system-ui,
      -apple-system,
      sans-serif;
    background: #0f172a;
    color: #e2e8f0;
  }

  .screen {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
  }

  .card {
    width: 100%;
    max-width: 24rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 0.75rem;
    padding: 2rem;
  }

  .card.wide {
    max-width: 32rem;
  }

  .card h1 {
    margin: 0 0 0.25rem;
    font-size: 1.25rem;
  }

  .hint {
    margin: 0;
    color: #94a3b8;
    font-size: 0.875rem;
  }

  .card label {
    font-size: 0.8rem;
    color: #cbd5e1;
  }

  .card input {
    padding: 0.5rem 0.625rem;
    border-radius: 0.5rem;
    border: 1px solid #475569;
    background: #0f172a;
    color: inherit;
    font-size: 0.9rem;
  }

  .card button {
    margin-top: 0.5rem;
    padding: 0.6rem;
    border-radius: 0.5rem;
    border: none;
    background: #34d399;
    color: #0f172a;
    font-weight: 600;
    cursor: pointer;
  }

  .card button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error {
    margin: 0;
    color: #f87171;
    font-size: 0.85rem;
  }

  .switch {
    margin: 0.5rem 0 0;
    font-size: 0.85rem;
    color: #94a3b8;
    text-align: center;
  }

  .switch a {
    color: #34d399;
  }

  .home-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }

  .home-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .home-header button {
    width: auto;
    padding: 0.5rem 0.9rem;
    margin-top: 0;
    align-self: flex-start;
  }

  .home-header a {
    color: #34d399;
    font-size: 0.9rem;
    text-decoration: none;
  }

  .home-header a:hover {
    text-decoration: underline;
  }

  .role-badge {
    display: inline-block;
    margin-left: 0.5rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: #334155;
    color: #cbd5e1;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .role-badge.admin {
    background: #34d399;
    color: #0f172a;
  }

  .admin-actions {
    display: flex;
    gap: 0.5rem;
    margin-left: auto;
  }

  .admin-actions button {
    width: auto;
    padding: 0.35rem 0.7rem;
    margin-top: 0;
    font-size: 0.8rem;
  }

  .user-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .user-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.625rem;
    border-radius: 0.5rem;
    background: #0f172a;
  }

  .dot {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 999px;
    background: #64748b;
    flex-shrink: 0;
  }

  .dot.online {
    background: #34d399;
  }

  .username {
    font-weight: 600;
  }

  .handle {
    color: #94a3b8;
    font-size: 0.85rem;
  }
</style>
