import { useStore } from "@nanostores/solid";
import { $notification, clearNotification, NotificationType } from "../../stores/notificationState";
import { Show } from "solid-js";

export const NotificationPopup = () => {
    const notification = useStore($notification);

    return (
        <div>
            <Show when={notification().active && notification().type === NotificationType.Success}>
                <div aria-live="assertive" class="pointer-events-none fixed inset-0 flex items-end px-4 py-6 sm:items-start sm:p-6">
                    <div class="flex w-full flex-col items-center space-y-4 sm:items-end">
                        <div class="pointer-events-auto group w-full max-w-sm overflow-hidden rounded-lg bg-emerald-600 shadow-lg">
                            <div class="p-4">
                                <div class="flex items-start">
                                    <div class="flex-shrink-0 my-auto">
                                        <svg xmlns="http://www.w3.org/2000/svg"
                                            width="20"
                                            height="20"
                                            fill="currentColor"
                                            class="bi bi-check-circle-fill text-lg mr-4 text-white"
                                            viewBox="0 0 16 16">
                                            <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0m-3.97-3.03a.75.75 0 0 0-1.08.022L7.477 9.417 5.384 7.323a.75.75 0 0 0-1.06 1.06L6.97 11.03a.75.75 0 0 0 1.079-.02l3.992-4.99a.75.75 0 0 0-.01-1.05z" />
                                        </svg>
                                    </div>
                                    <div class="ml-3 w-0 flex-1 pt-0.5">
                                        <p class="text-sm font-medium text-white">Success!</p>
                                        <p class="mt-1 text-sm text-white/80">{notification().message}</p>
                                    </div>
                                    <div class="ml-4 flex flex-shrink-0">
                                        <button type="button"
                                            onClick={clearNotification}
                                            class="inline-flex rounded-md">
                                            <span class="sr-only">Close</span>
                                            <svg xmlns="http://www.w3.org/2000/svg"
                                                width="12"
                                                height="12"
                                                fill="currentColor"
                                                class="bi bi-x-lg stroke-1 stroke-white invisible group-hover:visible"
                                                viewBox="0 0 16 16">
                                                <path d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z" />
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when={notification().active && notification().type !== NotificationType.Success}>
                <div
                    class="pointer-events-none fixed inset-x-0 bottom-0 sm:px-6 sm:pb-5 lg:px-8 ml-80"
                >
                    <Show when={notification().type === NotificationType.Banner}>
                        <div
                            class="pointer-events-auto flex items-center justify-between gap-x-6 bg-indigo-700 px-6 py-2.5 sm:py-3 sm:pl-4 sm:pr-3.5"
                        >
                            <i class="bi-exclamation-circle-fill text-lg mr-4 text-white"></i>
                            <p class="text-sm leading-6 text-white">
                                {notification().message}
                            </p>
                            <button
                                type="button"
                                onClick={clearNotification}
                                class="-m-3 flex-none p-3 focus-visible:outline-offset-[-4px]"
                            >
                                <span class="sr-only">Dismiss</span>
                                <i class="bi-x-lg text-white"></i>
                            </button>
                        </div>
                    </Show>
                    <Show when={notification().type === NotificationType.Error}>
                        <div
                            class="pointer-events-auto group flex items-center justify-between gap-x-6 bg-rose-600/70 px-6 py-2.5 sm:py-3 sm:pl-4 sm:pr-3.5"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg"
                                width="20"
                                height="20"
                                fill="currentColor"
                                class="bi bi-exclamation-circle-fill text-lg mr-4 text-white" viewBox="0 0 16 16">
                                <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0M8 4a.905.905 0 0 0-.9.995l.35 3.507a.552.552 0 0 0 1.1 0l.35-3.507A.905.905 0 0 0 8 4m.002 6a1 1 0 1 0 0 2 1 1 0 0 0 0-2" />
                            </svg>
                            <p class="text-sm leading-6 text-white">
                                {notification().message}
                            </p>
                            <button
                                type="button"
                                onClick={clearNotification}
                                class="invisible group-hover:visible -m-3 flex-none p-3 focus-visible:outline-offset-[-4px]"
                            >
                                <span class="sr-only">Dismiss</span>
                                <svg xmlns="http://www.w3.org/2000/svg"
                                    width="12"
                                    height="12"
                                    fill="currentColor"
                                    class="bi bi-x-lg stroke-1 stroke-white"
                                    viewBox="0 0 16 16">
                                    <path d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z" />
                                </svg>
                            </button>
                        </div>
                    </Show>
                    <Show when={notification().type === NotificationType.Warning}>
                        <div
                            class="pointer-events-auto flex items-center justify-between gap-x-6 bg-yellow-300/30 border-l-4 border-yellow-400 px-6 py-2.5 sm:py-3 sm:pl-4 sm:pr-3.5"
                        >
                            <i class="bi-exclamation-circle-fill text-lg mr-4 text-yellow-400"></i>
                            <p class="text-sm leading-6 text-yellow-700">
                                {notification().message}
                            </p>
                            <button
                                type="button"
                                onClick={clearNotification}
                                class="-m-3 flex-none p-3 focus-visible:outline-offset-[-4px]"
                            >
                                <span class="sr-only">Dismiss</span>
                                <i class="bi-x-lg text-yellow-700"></i>
                            </button>
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    );
};
