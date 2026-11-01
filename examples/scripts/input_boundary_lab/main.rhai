fn on_post_sim(ctx) {
    let panel = ctx.ui.panel("input_boundary", "Input Boundary Lab");
    panel.heading("mod_input", "MOD-RUNTIME INPUT");

    if ctx.input.available() {
        panel.status("sample_state_active", "STABLE · F1 overlay capture is sampling keyboard/mouse", "stable");
        let q = "up";
        let r = "up";
        let mouse = "up";
        if ctx.input.down("q") { q = "down"; }
        if ctx.input.down("r") { r = "down"; }
        if ctx.input.down("mouse1") { mouse = "down"; }
        panel.label("q_state", "Q  " + q);
        panel.label("r_state", "R  " + r);
        panel.label("mouse_state", "Left mouse  " + mouse);
        let q_pressed = "no";
        if ctx.input.just_pressed("q") { q_pressed = "yes"; }
        panel.label("q_press", "Q pressed  " + q_pressed);
    } else {
        panel.status("sample_state_inactive", "LIMITED · press F1 to capture overlay input; sampling is otherwise idle", "problematic");
    }

    panel.heading("native_input", "MAD MAX INPUT");
    panel.status("native_block", "EXPERIMENTAL · F1 captures keyboard/mouse messages; native actions are not individually mapped", "experimental");
    panel.label("domain_note", "F1 blocks overlay keyboard/mouse messages from normal game handling; XInput/gamepad is unverified.");
    panel.label("keybind_menu", "WIP · CKeyboardRouter / CInputActionOverride names found; Controls-menu registration path unknown.");
    ctx.ui.publish(panel);
}
