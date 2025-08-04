import {
    createComputed,
    createEffect,
    createSignal,
    mergeProps,
    onCleanup,
    untrack,
} from 'solid-js';
import makeTippy, { type Instance, type Props } from 'tippy.js';
import 'tippy.js/dist/tippy.css';

export interface TippyOptions {
    disabled?: boolean;
    hidden?: boolean;
    props?: Partial<Props>;
}

const defaultProps: Partial<Props> = {
    delay: [500, 0],
}

export function tippy<T extends Element>(
    target: T,
    opts: () => TippyOptions | undefined,
): void {
    createEffect(() => {
        const options = opts();
        const instance = makeTippy(target, untrack(() => mergeProps(defaultProps, options?.props)));

        createComputed(() => {
            if (options?.disabled) {
                instance.disable();
            } else {
                instance.enable();
            }
        });

        createComputed(() => {
            if (options?.hidden) {
                instance.show();
            } else {
                instance.hide();
            }
        });

        createComputed(() => {
            instance.setProps({
                ...(mergeProps(defaultProps, options?.props)),
            });
        });

        onCleanup(() => {
            instance.destroy();
        });
    });
}

export function useTippy<T extends Element>(
    target: () => T | undefined | null,
    options?: TippyOptions,
): () => Instance | undefined {
    const [current, setCurrent] = createSignal<Instance>();

    createEffect(() => {
        const currentTarget = target();
        if (currentTarget) {
            const instance = makeTippy(currentTarget, untrack(() => options?.props));

            setCurrent(instance);

            createComputed(() => {
                if (options?.disabled) {
                    instance.disable();
                } else {
                    instance.enable();
                }
            });

            createComputed(() => {
                if (options?.hidden) {
                    instance.hide();
                } else {
                    instance.show();
                }
            });

            createComputed(() => {
                instance.setProps({
                    ...(options?.props ?? {}),
                });
            });

            onCleanup(() => {
                instance.destroy();
            });
        }
    });

    return () => current();
}
