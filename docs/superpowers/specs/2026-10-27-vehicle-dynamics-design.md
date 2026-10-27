# Vehicle Dynamics Readouts Design

## Intent

Expand the Vehicle debug section with two native dynamics values from `CVehicle`: angular velocity and velocity from the previous frame. Display each as a scalar magnitude to match the existing speed and acceleration readouts.

## Goals

- Read `CVehicle::GetAngularVelocity(CVector3f*)` and `CVehicle::GetVelocityLastFrame()` from the game callback.
- Present magnitudes with clearly named labels and no guessed physical units.
- Keep sampling in the existing `UpdatePostSim` callback and preserve the current sampling interval.
- Fail closed when the SDK-derived vtable entry or returned vector pointer is unreadable, or when values are non-finite.

## Non-goals

- Add other vehicle state fields in this slice.
- Change sampling cadence, UI layout, or player readouts.
- Launch the game as part of deployment.

## Approaches

- **Vehicle dynamics first (selected):** add angular velocity and previous-frame velocity. This directly extends the physics telemetry just added and is easy to compare with current native velocity.
- **Vehicle state first:** add grounded-wheel count, target height, trust, and state. This broadens the panel but mixes operating state with the current physics focus.

## Implementation notes

The pinned [`vehicle.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/vehicle.h) declares the two methods. Their candidate primary-vtable slots are 57 and 59, counted after the virtual declarations in [`pfxgo.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/pfxgo.h), [`physgo.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/physgo.h), and [`damageable.h`](https://github.com/gigaHours/mm_sdk/blob/f2dc640769c88aae9c3965a2aca0aa138f0e62ab/game/go/damageable.h). These are source declarations, not live-binary verification. Validate slot readability and executable targets before calling. `GetVelocityLastFrame` returns a pointer; validate that returned address is readable for a 3-float vector before copying it. Copy vectors during the callback and store only owned Rust values for the overlay. Calculate and display magnitudes, using neutral labels such as `Angular velocity magnitude` and `Previous-frame speed magnitude`; retain the existing unspecified game-unit convention.

## Verification and delivery

Format and build the Windows release. Do not add or run tests unless requested. Before installing, confirm Mad Max is closed, back up the current ASI, deploy the matching build, then commit and push the change. Do not launch the game.
