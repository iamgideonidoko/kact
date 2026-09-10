import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import mermaid from 'astro-mermaid';

export default defineConfig({
  site: 'https://iamgideonidoko.github.io',
  base: '/kact',
  integrations: [
    mermaid({ autoTheme: true, enableLog: false }),
    starlight({
      title: 'Kact',
      description: 'Kact: Keyboard-driven cursor ACTuator.',
      favicon: '/kact.svg',
      logo: { src: './public/kact.svg', alt: 'Kact cursor logo' },
      social: [{ icon: 'github', label: 'Kact on GitHub', href: 'https://github.com/iamgideonidoko/kact' }],
      editLink: { baseUrl: 'https://github.com/iamgideonidoko/kact/edit/main/docs/' },
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        { label: 'Start here', items: ['index', 'getting-started/introduction', 'getting-started/installation', 'getting-started/quick-start', 'getting-started/platform-setup'] },
        { label: 'Concepts', items: ['concepts/how-kact-works', 'concepts/movement-and-coordinates'] },
        { label: 'Guides', items: ['guides/keyboard-and-shell-workflows', 'guides/external-integrations', 'guides/login-startup'] },
        { label: 'Configuration', items: ['configuration/overview', 'configuration/keybindings'] },
        { label: 'Reference', items: ['reference/cli', 'reference/filesystem-and-environment'] },
        { label: 'Help', items: ['help/troubleshooting', 'help/faq'] },
        { label: 'Contributing', items: ['contributing/development', 'contributing/architecture', 'contributing/decisions', 'contributing/testing', 'contributing/security', 'contributing/changelog', 'contributing/releasing'] }
      ]
    })
  ]
});
