fn publish_event_inventory(ctx) {
    let panel = ctx.ui.panel("event_inventory", "EVENT INVENTORY");

    panel.heading("actions", "MAPPED ACTION REGISTRATIONS");
    panel.status("action_mapping", "Special-camera action registrations mapped; mod observation and dispatch unavailable", "experimental");
    panel.label("action_01", "__specialcamera_freefly");
    panel.label("action_02", "__specialcamera_cancel");
    panel.label("action_03", "__specialcamera_movie");
    panel.label("action_04", "__specialcamera_movie_reset");
    panel.label("action_05", "__specialcamera_soakfly");
    panel.label("action_06", "__specialcamera_debug");
    panel.label("action_07", "__specialcamera_debug_south");
    panel.label("action_08", "__specialcamera_debug_east");
    panel.label("action_09", "__specialcamera_set_transform");
    panel.label("action_10", "__specialcamera_input_enable");
    panel.label("action_11", "__specialcamera_input_disable");
    panel.label("action_12", "__specialcamera_toggle_follow_terrain");
    panel.status("unknown_hashes", "Two additional registered hashes have unknown names: 0x6527BBC9, 0x2382A6C0", "experimental");

    panel.heading("registry", "SHARED ACTION SYSTEM");
    panel.status("registry_mapping", "Registration, subscriber ownership, queued dispatch, and payload callback shapes mapped", "stable");
    panel.label("registry_register", "RegisterActionByHash · RegisterActionHandlerKeys");
    panel.label("registry_subscribe", "AddActionSubscriber · RemoveActionSubscriber");
    panel.label("registry_dispatch", "QueueOrDispatchActionEvent · DispatchActionPayload");
    panel.label("registry_callback", "InvokeActionSubscribers calls handler vtable slot 1 with registry and payload pointers");
    panel.status("payload_limit", "Payload layouts, lifetime, thread timing, and re-entrancy are not mapped; no safe script API", "experimental");

    panel.heading("publishers", "PUBLISHER SHAPES");
    panel.label("publisher_name", "DispatchNamedActionEvent · name-keyed payload");
    panel.label("publisher_u32", "QueueNamedActionWithU32Data · name plus copied 32-bit value");
    panel.label("publisher_transform", "PublishNamedActionWithTransform · camera transform payload");

    panel.heading("rtti", "RELATED RTTI LEADS · NOT LINKED TO THE REGISTRY");
    panel.label("rtti_router", "CEventRouter");
    panel.label("rtti_handler", "CGameEventHandler");
    panel.label("rtti_trigger", "CReceiveEventsTrigger");
    panel.label("rtti_stealth", "CEventStealthRouter");
    panel.label("rtti_queue", "CFreeEventDataQueue");
    panel.label("rtti_input", "CInputActionOverride");
    panel.label("rtti_system", "EventSystem");

    panel.label("source_note", "Research: gameobject-transform-and-camera-follow-2026-11-01.md");
    ctx.ui.publish(panel);
}
