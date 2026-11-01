fn on_post_sim(ctx) {
    if state.contains("published") {
        return;
    }

    publish_event_inventory(ctx);
    state.published = true;
}
