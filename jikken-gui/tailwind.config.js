/** @type {import('tailwindcss').Config} */
import formsPlugin from '@tailwindcss/forms'

export default {
	content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
	theme: {
		extend: {
			colors: {
				'neutral-850': '#1c1c1c'
			}
		},
	},
	plugins: [
		formsPlugin,
	],
}
