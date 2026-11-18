# PortalWorlds API reference

Every public class and member of the PortalWorlds plugin (module `PortalWorlds`, Unreal Engine 5.8), as declared in `Plugins/PortalWorlds/Source/PortalWorlds/Public/`. For how to use them, see the [Guide](Guide.md).

**Exposure** column:

- **BP callable:** a Blueprint node with an execution pin.
- **BP pure:** a Blueprint node without an execution pin.
- **BP read/write** and **BP read-only:** a property's Get/Set nodes.
- **BP event:** a bindable event dispatcher.
- **Editable:** shown in the Details panel. **Instance only** means it can be set on placed actors but not in class defaults.
- **C++ only:** not reachable from Blueprints. It's listed for completeness, since Hexfold is Blueprint-only.

Units are centimetres, and directions are in world space unless stated otherwise.

- [APortal](#aportal-portal)
- [UPortalTraversalComponent](#uportaltraversalcomponent-portal-traversal)
- [UPortalGrabComponent](#uportalgrabcomponent-portal-grab)
- [APortalPlayerCameraManager](#aportalplayercameramanager)
- [UPortalSubsystem](#uportalsubsystem-portal-subsystem)
- [Console variables](#console-variables)
- [Content assets](#content-assets)
- [Logging](#logging)
- [Conventions and constants](#conventions-and-constants)

---

## APortal (Portal)

`Portal.h` · Parent: `AActor` · Placeable

A portal surface. The surface lies in the actor's local XZ plane, centred on the actor origin, and spans ±`SurfaceHalfExtent` before actor scale. Its front normal is local **−Y**. The root component is the surface mesh (`SM_PortalQuad` with `M_Portal` by default), so the actor's scale is the surface's scale. Keep it uniform; only the sign of X may differ, which makes the portal mirroring.

Each portal registers with the [Portal Subsystem](#uportalsubsystem-portal-subsystem) on BeginPlay and unregisters on EndPlay.

**Warp.** Looking or walking through a portal into its target applies

```
Warp = this.LocalToWorld⁻¹ × Target.LocalToWorld
```

A point keeps its coordinates relative to the portal, with no 180° flip, so entering this portal's front leaves through the target's back. A size difference scales whatever passes through. A determinant sign difference (one portal with negative X scale) mirrors it.

### Properties

| Name (Blueprint) | Type | Default | Exposure | Description |
| --- | --- | --- | --- | --- |
| `FrontLink` (**Front Link**) | `APortal*` | None | Editable (instance only), BP read/write | The portal reached by looking or walking through the front side, the side the normal points to. None makes that side an ordinary surface. |
| `BackLink` (**Back Link**) | `APortal*` | None | Editable (instance only), BP read/write | The portal reached through the back side |
| `SurfaceHalfExtent` (**Surface Half Extent**) | `FVector2D` | (100, 100) | Editable, BP read-only | Half width (local X) and half height (local Z) of the portal rectangle, before actor scale. Must match the surface mesh. Used for traversal, visibility and distance tests. |

The surface mesh component, `Surface`, is visible in the Details panel but private.

### Functions

| Name (Blueprint) | Signature | Exposure | Description |
| --- | --- | --- | --- |
| **Connect** | `static void Connect(APortal* A, APortal* B)` | BP callable | Sets both links of A to B and both links of B to A |
| **Get Normal** | `FVector GetNormal() const` | BP pure | Unit normal of the front side: the actor's local −Y axis in world space, with scale (including a negative X) ignored |
| **Get Link For** | `APortal* GetLinkFor(const FVector& Point) const` | BP pure | The link a viewer at `Point` would look through: `FrontLink` if `Point` is on the front side, `BackLink` if behind, or None if it is on the plane |
| **Is Mirrored Link To** | `bool IsMirroredLinkTo(const APortal* Target) const` | BP pure | True if passing from this portal to `Target` mirrors space, because exactly one of the two has a negative scale determinant |
| — | `double SignedDistance(const FVector& Point) const` | C++ only | Signed distance from the portal plane, positive on the front side |
| — | `bool Intersects(const FVector& From, const FVector& To, const FVector& Bump) const` | C++ only | True if segment From→To crosses the portal rectangle, with the plane offset by `Bump` |
| — | `double DistanceTo(const FVector& Point) const` | C++ only | Distance from `Point` to the portal rectangle |
| — | `void GetCorners(FVector OutCorners[4]) const` | C++ only | World-space corners of the rectangle |
| — | `FMatrix GetWarpTo(const APortal* Target) const` | C++ only | The warp matrix into `Target`'s space (row-vector convention) |
| — | `UStaticMeshComponent* GetSurface() const` | C++ only | The surface mesh component |
| — | `void ShowTexture(UTexture* Texture, bool bMirrorU = false)` | C++ only | Points the surface material at a captured view (sets `PortalTex`, clears `Pink`, sets `MirrorU`). Called by the subsystem. |
| — | `void ShowPink()` | C++ only | Shows the material's flat `Pink` colour. Called by the subsystem at the recursion limit and in `Portal.Debug`. |

### Surface material contract

| Parameter | Type | Meaning |
| --- | --- | --- |
| `PortalTex` | Texture parameter | The view through the portal. Sample it at `ScreenPosition` → `ViewportUV`. |
| `Pink` | Scalar parameter | 1 = draw a flat colour instead of `PortalTex`. The default in `M_Portal` is 1. |
| `MirrorU` | Scalar parameter | 1 = sample at U' = 1 − U. Needed only on mirroring portals. |

---

## UPortalTraversalComponent (Portal Traversal)

`PortalTraversalComponent.h` · Parent: `UActorComponent` · Class group: Portals · Blueprint spawnable · Ticks in `TG_PostPhysics`, after character movement and before the camera update

Teleports its owner through portals. It behaves in one of two modes, depending on the owner:

**Pawn mode** (the owner is an `APawn`):

- Each frame, the path of the **Eye** component since the last frame is tested against every portal rectangle. The plane is moved 2 mm × scale towards the side the eye came from, so the eye never reaches the surface and the near plane never clips it.
- On a crossing, the pawn's location, control rotation and velocity are warped to the linked portal and pushed twice the bump past the exit plane, and the camera is flagged as cut.
- A scaled warp calls `ApplyScaleFactor`. A mirroring warp toggles `IsMirrored`.
- A second tick function runs in `TG_PrePhysics`, after the controller and before the movement component. While mirrored, it reflects pending movement input left-right (`bMirrorMovementInput`).

**Object mode** (any other owner, such as a physics prop):

- The tracked point is the root component's origin, so give objects a centred pivot.
- On a crossing, the whole actor transform is multiplied by the warp. A scaling portal resizes the object. A mirroring portal gives it a negative scale (stored as negative X) and rebuilds its render state.
- A simulating root keeps its linear and angular velocity (both warped), and its mass is recomputed after a size change.
- An object tagged with a space moves to the exit portal's space.
- While partway through, a crossing copy can be shown and both can be cut at the plane. See `bShowCopyWhileCrossing` and `bClipAtPortal`.

### Properties

| Name (Blueprint) | Type | Default | Exposure | Description |
| --- | --- | --- | --- | --- |
| `Eye` (**Eye**) | `USceneComponent*` | None | Editable, BP read/write | The point that must cross a portal. None uses the owner's camera component, or its root if it has no camera. |
| `OnPortalTraversed` (**On Portal Traversed**) | `FOnPortalTraversedSignature` | — | BP event | Fired after each trip. See [Events](#events). |
| `bMirrorMovementInput` (**Mirror Movement Input**) | `bool` | true | Editable, BP read/write | Pawns: while mirrored, reflect movement input left-right before the movement component uses it, so strafing right still moves right on screen |
| `bShowCopyWhileCrossing` (**Show Copy While Crossing**) | `bool` | true | Editable, BP read/write | Objects: while partway through, show a copy of the owner's static meshes at the linked portal. The copy is query-only and blocks pawns only (simple collision). A character walking into it pushes the real object, using its character movement's push settings. |
| `bClipAtPortal` (**Clip At Portal**) | `bool` | true | Editable, BP read/write | Objects: while partway through, cut the owner and its copy at the portal plane, so each part appears on one side only. The plane goes into custom primitive data `ClipPlaneDataIndex`..+3 of every primitive. Materials must use `MF_PortalClip`, and others ignore the cut. While cut, the owner also ignores `ECC_Pawn` for pawns standing on the cut-away side near the portal. |
| `ClipPlaneDataIndex` | `static constexpr int32` | 32 | C++ only | First of the four custom primitive data floats that hold the clip plane |

### Functions

| Name (Blueprint) | Signature | Exposure | Description |
| --- | --- | --- | --- |
| **Is Mirrored** | `bool IsMirrored() const` | BP pure | True while the owner is mirrored relative to the world, after an odd number of mirroring portals |
| **Set Mirrored** | `void SetMirrored(bool bNewMirrored)` | BP callable | Sets the mirrored state directly, for example on respawn. It changes the flag only, not the actor's scale. |
| **Refresh Mirrored Rendering** | `static void RefreshMirroredRendering(AActor* Actor)` | BP callable (static) | Rebuilds the render state of every primitive on `Actor`. Call it after flipping the sign of an actor's scale yourself. Without it, the mesh keeps its old face culling and renders inside out. Traversal calls it automatically. |
| **Get Traversal Count** | `int32 GetTraversalCount() const` | BP pure | The number of portals passed through since play began |
| **Reset Tracking** | `void ResetTracking()` | BP callable | Forgets the eye's previous position. Call it after any discontinuous move (respawn, scripted teleport) so the jump isn't taken as a portal crossing. |
| **Apply Scale Factor** | `void ApplyScaleFactor(double Factor)` | BP callable | Multiplies the actor scale by `Factor`. If the owner has a `CharacterMovementComponent`, it also multiplies `MaxWalkSpeed`, `MaxWalkSpeedCrouched`, `MaxAcceleration`, `BrakingDecelerationWalking`, `BrakingDecelerationFalling`, `MaxStepHeight`, `JumpZVelocity` and `GravityScale`. Scaling portals call it automatically. |
| — | `static UPortalTraversalComponent* FindForPlayer(const APlayerController* PC)` | C++ only | The traversal component on the player's view target or pawn, if any |
| — | `void StopIgnoringPawns()` | C++ only | Objects: restores the owner's `ECC_Pawn` response if a cut turned it off. Call it before changing that response yourself. |
| — | `AActor* GetVisibleCrossingCopy(FMatrix& OutWarp) const` | C++ only | Objects: the crossing copy currently shown, or null. `OutWarp` maps the owner onto the copy. |

### Events

**On Portal Traversed** — `FOnPortalTraversedSignature(APortal* FromPortal, APortal* ToPortal, float ScaleFactor)`

| Parameter | Description |
| --- | --- |
| **From Portal** | The portal entered |
| **To Portal** | The portal exited (From Portal's link on the side that was entered) |
| **Scale Factor** | Size change from the trip. 1 for a plain portal, 0.5 for one that halves. |

It is broadcast after the owner has been moved, scaled and, if applicable, mirrored.

---

## UPortalGrabComponent (Portal Grab)

`PortalGrabComponent.h` · Parent: `UActorComponent` · Class group: Portals · Blueprint spawnable · Ticks in `TG_PostPhysics`, after traversals

Picks up, carries and throws simulating physics objects, including through portals. Add it to the player pawn, next to its Portal Traversal component. It creates its own `UPhysicsHandleComponent` at BeginPlay.

- The hold point is computed in the player's frame (eye position, view yaw, size and mirrored state). It is warped by a "carry warp" that collects the portals between the player and the object: each trip the object makes is appended, and each trip the player makes is cancelled out. An object carried through a portal therefore changes together with the player.
- When the held object traverses, the handle re-grabs it in place, so the kinematic anchor doesn't sweep across the jump.
- Aim traces pass through up to two portals.
- While held, the object's `ECC_Pawn` response is set to Ignore, and it is restored on release.
- To pass through portals, held objects need their own Portal Traversal component and a centred pivot.

All distances are in cm **at the player's normal size**. They scale with the player.

### Properties

| Name (Blueprint) | Type | Default | Exposure | Description |
| --- | --- | --- | --- | --- |
| `GrabRange` (**Grab Range**) | `float` | 250 | Editable, BP read/write | How far away an object can be picked up |
| `HoldGap` (**Hold Gap**) | `float` | 50 | Editable, BP read/write | Gap between the eye and the held object's bounding sphere |
| `MaxObjectRadius` (**Max Object Radius**) | `float` | 90 | Editable, BP read/write | Largest bounding-sphere radius that can be picked up, relative to the player's size |
| `ThrowSpeed` (**Throw Speed**) | `float` | 700 | Editable, BP read/write | Throw speed in cm/s |
| `BreakDistance` (**Break Distance**) | `float` | 120 | Editable, BP read/write | The object is dropped if it stays this far from its hold point for 0.5 s, for example when stuck behind a wall. Right after a grab, the allowance starts larger, to cover the initial pull, and shrinks to this value. |

### Functions

| Name (Blueprint) | Signature | Exposure | Description |
| --- | --- | --- | --- |
| **Grab** | `bool Grab()` | BP callable | Picks up the object under the crosshair, also through portals. Returns true if something was picked up. |
| **Release** | `void Release()` | BP callable | Drops the held object |
| **Throw** | `void Throw()` | BP callable | Releases the held object with a velocity of `ThrowSpeed` along the view |
| **Toggle Grab** | `void ToggleGrab()` | BP callable | Release if holding, otherwise Grab |
| **Is Holding** | `bool IsHolding() const` | BP pure | Whether an object is held |
| **Get Held Component** | `UPrimitiveComponent* GetHeldComponent() const` | BP pure | The held primitive, or None |
| **Get Aimed Component** | `UPrimitiveComponent* GetAimedComponent() const` | BP pure | What **Grab** would pick up now, or None. Use it for interaction prompts. |
| **Get Held Relative Scale** | `float GetHeldRelativeScale() const` | BP pure | The held object's size relative to the player. It is 1 when both have been through the same portals. |
| — | `FTransform GetHoldTarget() const` | C++ only | Where the held object is being pulled to, in world space, on the object's side of any portal |

Log output (`LogPortalWorlds`, Log): `Grabbed <name> (relative size x)`, `Released <name>`, `Dropped <name>: it got stuck`.

---

## APortalPlayerCameraManager

`PortalPlayerCameraManager.h` · Parent: `APlayerCameraManager`

Set it, or a Blueprint subclass, as the player controller's **Player Camera Manager Class**. In Hexfold, `BP_FirstPersonCameraManager` derives from it.

| Override | Behaviour |
| --- | --- |
| `UpdateCamera(float DeltaTime)` | Calls `Super`, then `UPortalSubsystem::RenderPortals(PCOwner, GetCameraCacheView())`. The portal views are therefore built from the camera's final position for the frame, which removes any lag. |
| `ProcessViewRotation(float, FRotator&, FRotator& OutDeltaRot)` | Calls `UPortalSubsystem::MirrorViewRotationInput` first, which reverses yaw (and roll) input while the view target is mirrored, then `Super` |

It has no Blueprint-visible members of its own. A camera manager that doesn't derive from this class must make the same two calls in C++, so Blueprint-only projects must derive from it.

---

## UPortalSubsystem (Portal Subsystem)

`PortalSubsystem.h` · Parent: `UWorldSubsystem` · Exists only in **Game** and **PIE** worlds (not in the editor viewport) · Blueprint: **Get PortalSubsystem**

It owns the portal registry, the per-frame view tree, scene-capture components, render targets, the mirror-view post-process pass and spaces.

**Per frame** (from `RenderPortals`):

1. Build a tree of views. The root is the player's view. Each portal visible in a view (facing, clip plane, and screen rectangle within the parent portal's rectangle, all tested on the CPU) adds a child view, made by warping the camera through the portal and clipping at the exit portal. Portals in views at depth `MaxDepth − 1` are drawn pink.
2. Capture the views deepest-first into render targets, using the player's projection, aspect ratio and post-process settings. Before each capture, the portals visible in that view are pointed at their child views' targets.
3. Point the portals in the player's view at the depth-1 targets.
4. Adjust the global near plane (`Portal.DynamicNearPlane`) and the mirror-view pass.

Each capture clips at its exit portal using the global clip plane (`r.AllowGlobalClipPlane=1`). A mirrored view is captured with the matching unmirrored camera, and its image is flipped through `MirrorU` (parent mirrored XOR child mirrored).

### Blueprint functions

| Name (Blueprint) | Signature | Exposure | Description |
| --- | --- | --- | --- |
| **Get Portals** | `TArray<APortal*> GetAllPortals() const` | BP pure | All portals currently in play |
| **Get Current Space** | `int32 GetCurrentSpace() const` | BP pure | The space containing the player's camera, or −1 if the level doesn't use spaces |
| **Get Actor Space** | `static int32 GetActorSpace(const AActor* Actor)` | BP pure (static) | The space from the actor's first `PortalSpace.<n>` tag (in `Actor->Tags`), or −1 (every space) |
| **Set Actor Space** | `void SetActorSpace(AActor* Actor, int32 Space)` | BP callable | Replaces the actor's `PortalSpace.<n>` tag. −1 removes it. Spaces rebuild on the next frame. |
| **Refresh Spaces** | `void RefreshSpaces()` | BP callable | Re-reads every actor's space tag on the next frame. Call it after spawning or re-tagging actors. |
| **Get Nearest Portal Distance** | `double GetNearestPortalDistance(const FVector& Point) const` | BP pure | Distance from `Point` to the nearest portal rectangle, or a large number if there are none |
| **Get Last Capture Count** | `int32 GetLastCaptureCount() const` | BP pure | Portal captures rendered last frame |
| **Is View Mirrored** | `bool IsViewMirrored() const` | BP pure | Whether the player's view was shown mirrored last frame |

### C++-only members

| Signature | Description |
| --- | --- |
| `void RegisterPortal(APortal*)` / `void UnregisterPortal(APortal*)` | Called by `APortal` on BeginPlay and EndPlay |
| `void RenderPortals(APlayerController* PC, const FMinimalViewInfo& View)` | Renders all portal views for the frame. Call it once per frame after the camera update. |
| `static void MirrorViewRotationInput(const APlayerController* PC, FRotator& InOutDeltaRot)` | Reverses yaw and roll input while the player is mirrored |
| `const TArray<TWeakObjectPtr<APortal>>& GetPortals() const` | Raw registry. Blueprints use **Get Portals** (`GetAllPortals`). |
| `static const TCHAR* SpaceTagPrefix` | `"PortalSpace."` |
| `void SetActorClippedAtPortal(AActor* Actor, const APortal* Portal, const FVector& KeepNormal)` | Marks `Actor` as cut at `Portal`'s plane, keeping the `KeepNormal` side. Views through that portal from that side hide the actor. A null `Portal` clears the mark. Used by traversal. |
| `bool IsInCutAwayPart(const AActor* Actor, const FVector& Point) const` | True if `Point` is on the cut-away side of a cut actor |
| `void GetClippedActors(TArray<AActor*>& OutActors) const` | Actors currently cut at a portal |

---

## Console variables

| Variable | Type | Default | Effect |
| --- | --- | --- | --- |
| `Portal.MaxDepth` | int | 4 | Recursion depth. Views up to depth `MaxDepth − 1` are captured, and portals seen in the deepest views are drawn pink. |
| `Portal.MaxCaptures` | int | 32 | Upper limit on captures per frame. Further portals are drawn pink. |
| `Portal.ResolutionFalloff` | float | 1.0 | Render-target resolution multiplier per recursion level beyond the first |
| `Portal.DynamicNearPlane` | bool | 1 | Sets the engine's global near plane to clamp(distance to nearest portal / 2, 0.1 cm, 10 cm), so the camera can get right up to a portal. 0 leaves the near plane alone. |
| `Portal.Spaces` | bool | 1 | Render only the current space in each view. 0 shows every space everywhere, for debugging. |
| `Portal.Debug` | bool | 0 | Draw every portal with its flat `Pink` colour and skip all captures |
| `Portal.Hide` | bool | 0 | Hide portal surfaces and skip all captures |
| `Portal.ShowStats` | bool | 0 | On-screen view and capture counts and near-plane info |
| `Portal.ClipObjects` | bool | 1 | Cut objects and their crossing copies at the portal plane (`bClipAtPortal`). 0 turns cutting off everywhere. |

Required and recommended engine settings (`Config/DefaultEngine.ini`, `[/Script/Engine.RendererSettings]`):

| Setting | Value | |
| --- | --- | --- |
| `r.AllowGlobalClipPlane` | True | Required. Needs an editor restart. |
| `r.AllowOcclusionQueries` | False | Recommended |

---

## Content assets

All under `/PortalWorlds/` (`Plugins/PortalWorlds/Content/`).

| Asset | Purpose |
| --- | --- |
| `Meshes/SM_PortalQuad` | Default portal surface: a double-sided quad spanning ±100 cm in local XZ |
| `Materials/M_Portal` | Default portal surface material with `PortalTex`, `Pink` and `MirrorU`. Instance it for custom looks. |
| `Materials/MF_PortalClip` | Material function. Its **Mask** output goes to **Opacity Mask** (Masked blend mode) on every material of a traversing object, so the object can be cut at portal planes. It reads custom primitive data 32–35. |
| `Materials/M_PortalMirrorView` | Post-process material (after tonemapping) that flips the player's view while mirrored. Loaded by the subsystem. |
| `Materials/RT_PortalDefault` | Placeholder render target. It isn't referenced from the plugin's code; presumably it is `M_Portal`'s default `PortalTex` value. |

---

## Logging

Category: **`LogPortalWorlds`** (declared in `PortalWorlds.h`).

| Verbosity | Message |
| --- | --- |
| Error | `/PortalWorlds/Materials/M_PortalMirrorView is missing: the mirrored view can't be shown` |
| Warning | `r.AllowGlobalClipPlane is off: portal views will show geometry ...` (at play) |
| Log | `r.AllowOcclusionQueries is on: geometry may pop for a frame in portal views ...` |
| Warning | `Portal <name> is tagged PortalSpace.<n> but stands in space <m>'s area ...` (once per portal) |
| Warning | `<actor> has Clip At Portal on, but some of its materials don't support the cut ...` (editor and PIE, first cut) |
| Log | `Traversed <from> -> <to> (scale x...)` and `Object <name> traversed <from> -> <to> ...` |
| Log | `Grabbed`, `Released`, `Dropped ...: it got stuck` |
| Verbose | Capture and render-target creation and resizing |

Enable verbose output with the console command `log LogPortalWorlds Verbose`.

---

## Conventions and constants

| Item | Value |
| --- | --- |
| Portal plane | Local XZ, centred on the actor origin |
| Portal front | Local −Y |
| Default surface size | 200 × 200 cm (`SurfaceHalfExtent` = 100, 100) |
| Exit side | You exit through the **target's back** (no 180° flip) |
| Portal scale | Uniform. Only the sign of X may differ (negative = mirroring). |
| Traversal plane bump | 2 mm × scale, towards the side the tracked point came from |
| Space tag | `PortalSpace.<n>` in the actor's Tags. Untagged = every space. |
| Clip-plane custom primitive data | Indices 32–35 (`UPortalTraversalComponent::ClipPlaneDataIndex`) |
| Grab stuck timeout | 0.5 s beyond `BreakDistance` |
| Grab aim depth | Through up to two portals |
| Tick groups | Traversal: `TG_PostPhysics`. Input mirror: `TG_PrePhysics`, after the controller and before movement. Grab: `TG_PostPhysics`. Portal rendering: in `PlayerCameraManager::UpdateCamera`. |
