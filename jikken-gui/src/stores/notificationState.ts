import { atom } from "nanostores";

export enum NotificationType {
    Success,
    Warning,
    Error,
    Banner,
};

export type NotificationState = {
    active: boolean,
    type?: NotificationType,
    message?: string | HTMLElement,
    triggeredAt?: Date,
};

const initState: NotificationState = {
    active: false,
};

export const $notification = atom<NotificationState>(initState);

export async function triggerBanner(type: NotificationType, message: string | HTMLElement) {
    console.log("setting banner notification");
    $notification.set({
        active: true,
        type: type,
        message: message,
        triggeredAt: new Date(),
    });
}

export async function triggerNotification(type: NotificationType, message: string | HTMLElement, timeout: number | undefined = undefined) {
    console.log("setting notification");
    $notification.set({
        active: true,
        type: type,
        message: message,
        triggeredAt: new Date(),
    });

    let resolvedTimeout = timeout ? timeout : type === NotificationType.Success ? 3000 : 5000;
    await new Promise(f => setTimeout(f, resolvedTimeout));
    clearNotification();
};

export const clearNotification = () => {
    console.log("clearing notification");
    $notification.set({ active: false });
};
