# Free Camera Workbench

Load `freecam_workbench` from Dev Tools → Scripts. Enable the detached camera pose, then press F1 to capture keyboard/mouse input for the mod. Use WASD to move relative to the current view, Space/Left Ctrl for vertical movement, and arrow keys for yaw/pitch. The world position fields and rotation controls also work from the panel. Disable the pose to clear the override and restore the game's camera.

This uses the existing `CCameraControlManager::GetCameraMatrix` output override and is experimental. It moves the camera position away from Max but remains coupled to the game's camera system; it is not a fully replaced renderer camera. F1 captures keyboard/mouse window messages for the mod. Native gamepad/XInput suppression and Controls-menu keybind registration are not mapped.
