import 'solid-js';
import { TippyOptions } from './components/TippySolid';

declare module "solid-js" {
    namespace JSX {
        interface Directives {
            tippy: TippyOptions | undefined;
        }
    }
}
