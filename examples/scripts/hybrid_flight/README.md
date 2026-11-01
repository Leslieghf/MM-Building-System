# Hybrid Flight Workbench

Load `hybrid_flight` from Dev Tools → Scripts, press F1 to capture keyboard input, then enable **Vehicle flight** while in a vehicle. W/S moves forward/back relative to camera view, A/D strafes, and Space/Left Ctrl moves vertically. The script directly sets the vehicle velocity while the panel is enabled. With no movement key held, it requests zero velocity to hover. If F1 capture is inactive or camera data is unavailable, it sends no velocity write.

This is experimental velocity-based flight, not noclip or a teleport. Mad Max physics and collision remain active. Disable it before driving normally; the game retains the last velocity when you turn the panel off.

On foot, the panel reports Max movement as WIP and sends no vehicle write. No player transform/velocity setter or safe Havok teleport lifecycle is mapped. XInput/gamepad suppression and native Controls-menu keybind registration remain WIP; F1 captures the overlay's keyboard/mouse messages only.
