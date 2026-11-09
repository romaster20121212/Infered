 


























import java.lang.reflect.Method;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.Set;

 
public final class InferedFastScaffoldLogic {
     
    static final int NORMAL = 0;
    static final int EXPAND = 1;
     
    static final int TOWER_NONE = 0;
    static final int TOWER_MOTION = 1;

     
    static Object sessionPlayer, sessionWorld, supportPlayer, supportWorld;
    static boolean failed, resetRequested;
    static int supportY;
    static int inputTick, nextUseTick, placeHoldTick = -1;
     
     
    static final int PLACEMENT_INTERVAL = 2;
    private static boolean previousEdgeCrouch;
     


    private static boolean tickShadow;
    private static float tickAimYaw, tickAimPitch;
    private static boolean tickBodyArmed;
     
    private static boolean bodyArmWarned;
    static Object slotInventory;
    static int originalSlot = -1, assistedSlot = -1;
    static Object slotPlayer;
    static double towerJumpOffY = Double.NaN;
    static boolean towering;

    private static final int IDLE = 0, PREPARE_ROTATION = 1, PLACE = 2;
    static int phase = IDLE;
     

    private static int[] lockedDestination;

     
     
    static boolean edgeCrouch;
     

    private static int clickCrouchTick = -1;
    private static final int CLICK_CROUCH_GRACE = 3;
     
    private static boolean falling;
     
    private static final double FALLING_SPEED = .08;
     

    private static final double SUPPORT_DEPTH = .5;
     
    private static final double FOOT_CLEARANCE = .001;
     
    private static final double CORNER_SAMPLE = .01;
     


    private static final double NOMINAL_STEP = .3;
     
    private static final double CROUCH_HOLD_LOOKAHEAD = .5;

     
    private static final int MAX_LAST_PLACED = 4;
    private static final float DIRECTION_HYSTERESIS_DEGREES = 30f;
    private static final double SUPPORT_SURFACE_EPSILON = 1e-3;
    private static final double SUPPORT_OVERLAP_HYSTERESIS = .02;
    private static final ArrayDeque<int[]> lastPlacedBlocks = new ArrayDeque<>(MAX_LAST_PLACED);
    static int[] lastSupportBlock;
    static float lastDirectionAngle = Float.NaN;
     
    private static final int MAX_PLACEMENT_OFFSETS = 4;
    private static final double BOOTSTRAP_BACKOFF = .2;
    private static final double PREDICTION_CUTOFF = .05;
    private static final int WARMUP_PLACEMENTS = 2;
    private static final ArrayDeque<double[]> placementOffsets = new ArrayDeque<>(MAX_PLACEMENT_OFFSETS + 1);
    private static final int[][] NORMAL_OFFSETS = buildNormalOffsets();
    private static final String[] POSITION_METHODS = {"method_10263", "method_10264", "method_10260"};
    private static final String[] FACE_METHODS = {"method_10148", "method_10164", "method_10165"};
    private static Class<?> positionType, faceType, airStateType;
    private static Method[] positionAccess, faceAccess;
    private static Method airStateAccess;

    private InferedFastScaffoldLogic() {}

     

     
    static void beforeInput() {
        long settings = InferedFastScaffoldBridge.snapshot();
        inputTick++;
        if (placeHoldTick >= 0 && inputTick > placeHoldTick) {
            placeHoldTick = -1;
            InferedFastScaffoldBridge.AIM_ACTIVE = false;
        }
        if (resetRequested || !InferedFastScaffoldBridge.ACTIVE) {
            release();
            return;
        }
        if (failed) {
            cancel();
            return;
        }
        try {
            tick(settings);
             
             
             
            if (!tickBodyArmed) armBody();
        } catch (Exception e) {
            throw new IllegalStateException("Scaffold assist failed", InferedFastScaffoldBridge.cause(e));
        }
    }

     

    static void failed(Throwable cause) {
        failed = true;
        System.err.println("[Fast Scaffold] input assist stopped: " + cause);
        try {
            release();
        } catch (Throwable cleanup) {
             
             
            System.err.println("[Fast Scaffold] cleanup failed: " + cleanup);
        }
    }

    static void reset() {
        sessionPlayer = sessionWorld = supportPlayer = supportWorld = null;
        failed = resetRequested = false;
        phase = IDLE;
        inputTick = nextUseTick = 0;
        placeHoldTick = -1;
        originalSlot = assistedSlot = -1;
        slotInventory = slotPlayer = null;
        towerJumpOffY = Double.NaN;
        towering = false;
        lastPlacedBlocks.clear();
        placementOffsets.clear();
        lastSupportBlock = null;
        lastDirectionAngle = Float.NaN;
        InferedFastScaffoldBridge.AIM_ACTIVE = false;
        tickShadow = false;
        tickAimYaw = tickAimPitch = 0f;
        bodyArmWarned = false;
        edgeCrouch = previousEdgeCrouch = false;
        clickCrouchTick = -1;
        positionType = faceType = airStateType = null;
        positionAccess = faceAccess = null;
        airStateAccess = null;
        InferedFastScaffoldBridge.releaseSneak();
    }

     

    static void release() {
        edgeCrouch = previousEdgeCrouch = false;
        clickCrouchTick = -1;
        nextUseTick = inputTick;
        InferedFastScaffoldBridge.releaseSneak();
        try {
            restoreSlot();
        } catch (Exception e) {
            throw new IllegalStateException("Scaffold slot cleanup failed", InferedFastScaffoldBridge.cause(e));
        }
        placeHoldTick = -1;
        InferedFastScaffoldBridge.AIM_ACTIVE = false;
        clearRotationState();
        towerJumpOffY = Double.NaN;
        towering = false;
        lastPlacedBlocks.clear();
        placementOffsets.clear();
        lastSupportBlock = null;
        lastDirectionAngle = Float.NaN;
        resetRequested = false;
    }

     

    private static void tick(long settings) throws Exception {
         
         
        previousEdgeCrouch = edgeCrouch;
        edgeCrouch = false;
         
         
         
        tickShadow = InferedFastScaffoldBridge.SHADOW_ACTIVE && InferedFastScaffoldBridge.SILENT_AIM
            && InferedFastScaffoldBridge.AIM_ACTIVE;
        tickBodyArmed = false;
        tickAimYaw = InferedFastScaffoldBridge.SHADOW_YAW;
        tickAimPitch = InferedFastScaffoldBridge.SHADOW_PITCH;
        try {
            plan(settings);
        } finally {
             
             
             
            applyEdgeCrouch();
        }
    }

     






    private static boolean armBody() {
        if (!tickShadow || !InferedFastScaffoldBridge.AIM_ACTIVE
            || !InferedFastScaffoldBridge.SILENT_AIM || InferedFastScaffoldBridge.BLOCKED) return false;
        try {
            tickBodyArmed = InferedFastScaffoldBridge.armBody(tickAimYaw, tickAimPitch);
            return tickBodyArmed;
        } catch (Throwable t) {
            if (!bodyArmWarned) {
                bodyArmWarned = true;
                System.err.println("[Fast Scaffold] body aim unavailable; skipping silent placement: "
                    + InferedFastScaffoldBridge.unwrap(t));
            }
            return false;
        }
    }

    private static void plan(long settings) throws Exception {
        int technique = InferedFastScaffoldBridge.technique(settings);
        int tower = InferedFastScaffoldBridge.tower(settings);
        int length = InferedFastScaffoldBridge.expandLength(settings);
        int delay = InferedFastScaffoldBridge.delay(settings);
        float minDist = InferedFastScaffoldBridge.minDistance();
        boolean autoBlock = InferedFastScaffoldBridge.autoBlock(settings);

        Object client = InferedFastScaffoldBridge.client;
        Object p = InferedFastScaffoldBridge.playerField.get(client);
        Object world = InferedFastScaffoldBridge.worldField.get(client);
        if (slotInventory != null && (p != slotPlayer || !autoBlock)) restoreSlot();
        if (phase == PREPARE_ROTATION && (p != sessionPlayer || world != sessionWorld)) {
            clearRotationState();
            return;
        }
        if (p == null || world == null) {
            cancel();
            return;
        }
        if (p != supportPlayer || world != supportWorld) {
            supportPlayer = p;
            supportWorld = world;
            supportY = (int) Math.floor((double) InferedFastScaffoldBridge.positionY.invoke(p) + .001) - 1;
            towerJumpOffY = Double.NaN;
            lastPlacedBlocks.clear();
            placementOffsets.clear();
            lastSupportBlock = null;
            lastDirectionAngle = Float.NaN;
        }
        if (!InferedFastScaffoldBridge.eligible(p)) {
            cancel();
            return;
        }
        Object stack = selectStack(p, false, autoBlock);
        if (stack == null) {
            cancel();
            return;
        }

        float currentYaw = (float) InferedFastScaffoldBridge.yaw.invoke(p);
        float currentPitch = (float) InferedFastScaffoldBridge.pitch.invoke(p);
        float aimYawNow = aimYaw(p), aimPitchNow = aimPitch(p);
        if (!Float.isFinite(currentYaw) || !Float.isFinite(currentPitch)
            || !Float.isFinite(aimYawNow) || !Float.isFinite(aimPitchNow)) {
            cancel();
            return;
        }
        if (phase == IDLE && (boolean) InferedFastScaffoldBridge.grounded.invoke(p)) {
            supportY = (int) Math.floor((double) InferedFastScaffoldBridge.positionY.invoke(p) + .001) - 1;
        }

        float forward = (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.forwardKey) ? 1 : 0)
            - (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.backKey) ? 1 : 0);
        float strafe = (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.leftKey) ? 1 : 0)
            - (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.rightKey) ? 1 : 0);
        boolean jumpHeld = InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.jumpKey);
        towering = tower == TOWER_MOTION && jumpHeld;
        if (towering) {
            supportY = (int) Math.floor((double) InferedFastScaffoldBridge.positionY.invoke(p) + .001) - 1;
             
            towerMotion(p, world, jumpHeld);
        } else {
            towerJumpOffY = Double.NaN;
        }

         
         
         
         
         
         
         
         
         
        double lookahead = previousEdgeCrouch ? CROUCH_HOLD_LOOKAHEAD : NOMINAL_STEP;
        edgeCrouch = !towering && InferedFastScaffoldBridge.edgeSafety(settings)
            && (boolean) InferedFastScaffoldBridge.grounded.invoke(p)
            && stepLeavesGround(p, world, currentYaw, (int) forward, (int) strafe, lookahead);

         
         
         
         
         
         
         
         
         
        boolean moving = forward != 0 || strafe != 0;
        boolean silentAim = InferedFastScaffoldBridge.SILENT_AIM;
        if (!silentAim) {
             
            InferedFastScaffoldBridge.AIM_ACTIVE = false;
            if (phase == PREPARE_ROTATION) {
                 
                clearRotationState();
            }
            aimYawNow = currentYaw;
            aimPitchNow = currentPitch;
        }

         
         
        int cooldown = InferedFastScaffoldBridge.cooldownField.getInt(client);
        boolean mayPlace = cooldown == 0 && inputTick >= nextUseTick;
        if (!moving && !towering) {
            cancel();
            return;
        }
        double range = (double) InferedFastScaffoldBridge.reach.invoke(p);
        if (!Double.isFinite(range) || range <= 0) {
            cancel();
            return;
        }

         
         
        Line line = optimalLine(p, world, currentYaw, forward, strafe);
        double[] predicted = predictedPlacement(p, world, line);
         
        Object motion = InferedFastScaffoldBridge.deltaMovement.invoke(p);
        falling = (double) InferedFastScaffoldBridge.vy.invoke(motion) < -FALLING_SPEED;
        double px = predicted != null ? predicted[0] : (double) InferedFastScaffoldBridge.positionX.invoke(p);
        double py = predicted != null ? predicted[1] : (double) InferedFastScaffoldBridge.positionY.invoke(p);
        double pz = predicted != null ? predicted[2] : (double) InferedFastScaffoldBridge.positionZ.invoke(p);
        int[][] candidates = technique == EXPAND
            ? expandCandidates(px, py, pz, inputAngle(currentYaw, forward, strafe), length)
            : normalCandidates(px, py, pz, line);
        if (technique == NORMAL && !needsSupport(world, px, py, pz)) {
            clearRotationState();
            return;
        }

        if (phase != IDLE && phase != PREPARE_ROTATION) return;
        Object hit = vanillaRaycast(p, range, aimYawNow, aimPitchNow);
        int[] currentTarget = matchingTarget(hit, candidates, world);
        if (mayPlace && currentTarget != null && canPlaceAt(p, hit, currentTarget, stack)
            && validCrosshairTarget(p, hit, minDist)) {
            phase = PLACE;
            place(p, hit, currentTarget, autoBlock, delay);
            return;
        }
        if (!silentAim) return;
        FaceTarget target = findBestTarget(p, world, candidates, range, aimYawNow, aimPitchNow, stack);
        if (target == null) {
            if (phase == PREPARE_ROTATION) clearRotationState();
            return;
        }
        if (phase == IDLE) beginRotation(p, world);
        InferedFastScaffoldBridge.AIM_YAW = target.yaw;
        InferedFastScaffoldBridge.AIM_PITCH = target.pitch;
        InferedFastScaffoldBridge.AIM_ACTIVE = true;
    }

    private static void beginRotation(Object p, Object world) {
        sessionPlayer = p;
        sessionWorld = world;
        phase = PREPARE_ROTATION;
    }

    private static float aimYaw(Object p) throws Exception {
        return tickShadow ? tickAimYaw : (float) InferedFastScaffoldBridge.yaw.invoke(p);
    }

    private static float aimPitch(Object p) throws Exception {
        return tickShadow ? tickAimPitch : (float) InferedFastScaffoldBridge.pitch.invoke(p);
    }

    static void clearRotationState() {
        phase = IDLE;
        sessionPlayer = sessionWorld = null;
        lockedDestination = null;
        if (inputTick > placeHoldTick) InferedFastScaffoldBridge.AIM_ACTIVE = false;
    }

    private static void cancel() {
        clearRotationState();
    }

     

     









    private static void applyEdgeCrouch() {
        try {
            boolean armed = InferedFastScaffoldBridge.ACTIVE && !InferedFastScaffoldBridge.BLOCKED
                && !InferedFastScaffoldBridge.MENU_OPEN;
            boolean want = armed && (edgeCrouch || clickCrouch());
            InferedFastScaffoldBridge.holdSneak(want);
        } catch (Exception e) {
            InferedFastScaffoldBridge.releaseSneak();
            System.err.println("[Fast Scaffold] edge crouch failed: " + InferedFastScaffoldBridge.cause(e));
        }
    }

     













    private static boolean clickCrouch() throws Exception {
        Object client = InferedFastScaffoldBridge.client;
        Object p = InferedFastScaffoldBridge.playerField.get(client);
        if (p == null || InferedFastScaffoldBridge.worldField.get(client) == null) return false;
        if (InferedFastScaffoldBridge.screenField.get(client) != null) return false;
        if (!InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.forwardKey)
            || InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.jumpKey)
            || !(boolean) InferedFastScaffoldBridge.grounded.invoke(p)) return false;
        if (InferedFastScaffoldBridge.clicking()) clickCrouchTick = inputTick;
         
        int pressed = clickCrouchTick;
        return pressed >= 0 && inputTick >= pressed && inputTick - pressed <= CLICK_CROUCH_GRACE;
    }

     











    private static boolean stepLeavesGround(Object p, Object world, float yawValue, int forward, int strafe,
                                            double lookahead)
        throws Exception {
        if (forward == 0 && strafe == 0) return false;
        Object motion = InferedFastScaffoldBridge.deltaMovement.invoke(p);
        double mx = (double) InferedFastScaffoldBridge.vx.invoke(motion);
        double mz = (double) InferedFastScaffoldBridge.vz.invoke(motion);
         
         
        double stepLength = Math.max(lookahead, Math.hypot(mx, mz));
        double inputLength = Math.hypot(forward, strafe);
        double radians = Math.toRadians(yawValue);
        double x = strafe / inputLength * stepLength, z = forward / inputLength * stepLength;
        double dx = x * Math.cos(radians) - z * Math.sin(radians);
        double dz = z * Math.cos(radians) + x * Math.sin(radians);
        double px = (double) InferedFastScaffoldBridge.positionX.invoke(p) + dx;
        double py = (double) InferedFastScaffoldBridge.positionY.invoke(p);
        double pz = (double) InferedFastScaffoldBridge.positionZ.invoke(p) + dz;
        Object box = InferedFastScaffoldBridge.boundingBox.invoke(p);
        double halfWidth = ((double) InferedFastScaffoldBridge.boxMaxX.get(box)
            - (double) InferedFastScaffoldBridge.boxMinX.get(box)) / 2;
        for (double cornerX : new double[] {px - halfWidth, px + halfWidth}) {
            for (double cornerZ : new double[] {pz - halfWidth, pz + halfWidth}) {
                if (supported(world, cornerX, py, cornerZ)) return false;
            }
        }
        return true;
    }

     

    private static boolean supported(Object world, double x, double y, double z) throws Exception {
        double minX = x - CORNER_SAMPLE, maxX = x + CORNER_SAMPLE;
        double minZ = z - CORNER_SAMPLE, maxZ = z + CORNER_SAMPLE;
        double minY = y - SUPPORT_DEPTH, maxY = y - FOOT_CLEARANCE;
        int fromX = (int) Math.floor(minX), toX = (int) Math.floor(maxX - 1e-9);
        int fromY = (int) Math.floor(minY), toY = (int) Math.floor(maxY - 1e-9);
        int fromZ = (int) Math.floor(minZ), toZ = (int) Math.floor(maxZ - 1e-9);
        for (int bx = fromX; bx <= toX; bx++) {
            for (int bz = fromZ; bz <= toZ; bz++) {
                for (int by = fromY; by <= toY; by++) {
                    Object pos = InferedFastScaffoldBridge.blockPos.newInstance(bx, by, bz);
                    Object state = InferedFastScaffoldBridge.air.invoke(world, pos);
                    Object shape = InferedFastScaffoldBridge.collisionShape.invoke(state, world, pos);
                    if (Boolean.TRUE.equals(InferedFastScaffoldBridge.shapeEmpty.invoke(shape))) continue;
                    for (Object part : (List<?>) InferedFastScaffoldBridge.shapeBoxes.invoke(shape)) {
                        if (bx + (double) InferedFastScaffoldBridge.boxMinX.get(part) < maxX
                            && bx + (double) InferedFastScaffoldBridge.boxMaxX.get(part) > minX
                            && by + (double) InferedFastScaffoldBridge.boxMinY.get(part) < maxY
                            && by + (double) InferedFastScaffoldBridge.boxMaxY.get(part) > minY
                            && bz + (double) InferedFastScaffoldBridge.boxMinZ.get(part) < maxZ
                            && bz + (double) InferedFastScaffoldBridge.boxMaxZ.get(part) > minZ) {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }

     

     
    static final class Line {
        final double x, y, z, dx, dz;

        Line(double x, double y, double z, double dx, double dz) {
            this.x = x;
            this.y = y;
            this.z = z;
            this.dx = dx;
            this.dz = dz;
        }

        double distanceSqr(double px, double pz) {
            double ox = px - x, oz = pz - z;
            double projected = ox * dx + oz * dz;
            double ex = ox - dx * projected, ez = oz - dz * projected;
            return ex * ex + ez * ez;
        }

        double[] nearest(double px, double py, double pz) {
            double projected = (px - x) * dx + (pz - z) * dz;
            return new double[] {x + dx * projected, py, z + dz * projected};
        }
    }

     
    static final class SupportCandidate {
        final int[] pos;
        final double surfaceDelta;
        final double overlapArea;
        final double distanceSqr;

        SupportCandidate(int[] pos, double surfaceDelta, double overlapArea, double distanceSqr) {
            this.pos = pos;
            this.surfaceDelta = surfaceDelta;
            this.overlapArea = overlapArea;
            this.distanceSqr = distanceSqr;
        }

        boolean beats(SupportCandidate other) {
            if (surfaceDelta + SUPPORT_SURFACE_EPSILON < other.surfaceDelta) return true;
            if (other.surfaceDelta + SUPPORT_SURFACE_EPSILON < surfaceDelta) return false;
            if (overlapArea > other.overlapArea + SUPPORT_OVERLAP_HYSTERESIS) return true;
            if (overlapArea + SUPPORT_OVERLAP_HYSTERESIS < other.overlapArea) return false;
            return distanceSqr < other.distanceSqr;
        }

        boolean stableAgainst(SupportCandidate best) {
            if (surfaceDelta > best.surfaceDelta + SUPPORT_SURFACE_EPSILON) return false;
            return !(overlapArea + SUPPORT_OVERLAP_HYSTERESIS < best.overlapArea);
        }
    }

     
    private static float inputAngle(float yawValue, float forward, float strafe) {
        if (forward == 0 && strafe == 0) return yawValue;
        double radians = Math.toRadians(yawValue);
         
        double x = strafe * Math.cos(radians) - forward * Math.sin(radians);
        double z = forward * Math.cos(radians) + strafe * Math.sin(radians);
        return (float) Math.toDegrees(Math.atan2(-x, z));
    }

     

    private static double[] chooseDirection(float angle) {
        float chosen = lastDirectionAngle;
        boolean reuse = !Float.isNaN(chosen) && degreesDifferenceAbs(angle, chosen) <= DIRECTION_HYSTERESIS_DEGREES;
        if (!reuse) {
            double slot = Math.round(angle / 180.0 * 4 + 4);
            chosen = (float) wrapDegrees((slot - 4) / 4.0 * 180.0);
            lastDirectionAngle = chosen;
        }
        double radians = Math.toRadians(chosen);
        return new double[] {-Math.sin(radians), Math.cos(radians)};
    }

    private static double degreesDifferenceAbs(float a, float b) {
        double difference = Math.abs(a - b) % 360.0;
        return difference > 180.0 ? 360.0 - difference : difference;
    }

    private static double wrapDegrees(double value) {
        double wrapped = value % 360.0;
        if (wrapped >= 180.0) wrapped -= 360.0;
        if (wrapped < -180.0) wrapped += 360.0;
        return wrapped;
    }

     
    private static List<SupportCandidate> supportCandidates(Object p, Object world) throws Exception {
        List<SupportCandidate> candidates = new ArrayList<>();
        Object box = InferedFastScaffoldBridge.boundingBox.invoke(p);
        double minX = (double) InferedFastScaffoldBridge.boxMinX.get(box);
        double minY = (double) InferedFastScaffoldBridge.boxMinY.get(box);
        double minZ = (double) InferedFastScaffoldBridge.boxMinZ.get(box);
        double maxX = (double) InferedFastScaffoldBridge.boxMaxX.get(box);
        double maxZ = (double) InferedFastScaffoldBridge.boxMaxZ.get(box);
        int y = (int) Math.floor(minY) - 1;
        for (int x = (int) Math.floor(minX); x <= (int) Math.floor(maxX); x++) {
            for (int z = (int) Math.floor(minZ); z <= (int) Math.floor(maxZ); z++) {
                Object pos = InferedFastScaffoldBridge.blockPos.newInstance(x, y, z);
                Object state = InferedFastScaffoldBridge.air.invoke(world, pos);
                Object shape = InferedFastScaffoldBridge.collisionShape.invoke(state, world, pos);
                if (Boolean.TRUE.equals(InferedFastScaffoldBridge.shapeEmpty.invoke(shape))) continue;
                double bestSurface = Double.POSITIVE_INFINITY, overlapOnBest = 0;
                for (Object part : (List<?>) InferedFastScaffoldBridge.shapeBoxes.invoke(shape)) {
                    double pMinX = x + (double) InferedFastScaffoldBridge.boxMinX.get(part);
                    double pMaxX = x + (double) InferedFastScaffoldBridge.boxMaxX.get(part);
                    double pMaxY = y + (double) InferedFastScaffoldBridge.boxMaxY.get(part);
                    double pMinZ = z + (double) InferedFastScaffoldBridge.boxMinZ.get(part);
                    double pMaxZ = z + (double) InferedFastScaffoldBridge.boxMaxZ.get(part);
                    double overlapX = Math.min(maxX, pMaxX) - Math.max(minX, pMinX);
                    double overlapZ = Math.min(maxZ, pMaxZ) - Math.max(minZ, pMinZ);
                    if (overlapX <= 0 || overlapZ <= 0) continue;
                    double delta = Math.abs(minY - pMaxY), area = overlapX * overlapZ;
                    if (delta + SUPPORT_SURFACE_EPSILON < bestSurface) {
                        bestSurface = delta;
                        overlapOnBest = area;
                    } else if (Math.abs(delta - bestSurface) <= SUPPORT_SURFACE_EPSILON) {
                        overlapOnBest += area;
                    }
                }
                if (!Double.isFinite(bestSurface)) continue;
                double centreX = x + .5, centreZ = z + .5;
                double dx = (double) InferedFastScaffoldBridge.positionX.invoke(p) - centreX;
                double dz = (double) InferedFastScaffoldBridge.positionZ.invoke(p) - centreZ;
                candidates.add(new SupportCandidate(new int[] {x, y, z}, bestSurface, overlapOnBest, dx * dx + dz * dz));
            }
        }
        return candidates;
    }

     

    private static SupportCandidate supportReference(Object p, Object world) throws Exception {
        List<SupportCandidate> candidates = supportCandidates(p, world);
        if (candidates.isEmpty()) {
            lastSupportBlock = null;
            return null;
        }
        SupportCandidate best = candidates.get(0);
        for (SupportCandidate candidate : candidates) if (candidate.beats(best)) best = candidate;
        SupportCandidate previousPlaced = findCandidate(candidates, lastPlacedBlocks.peekLast());
        SupportCandidate previousSupport = findCandidate(candidates, lastSupportBlock);
        SupportCandidate chosen = previousPlaced != null && previousPlaced.stableAgainst(best) ? previousPlaced
            : previousSupport != null && previousSupport.stableAgainst(best) ? previousSupport
            : best;
        lastSupportBlock = chosen.pos;
        return chosen;
    }

    private static SupportCandidate findCandidate(List<SupportCandidate> candidates, int[] pos) {
        if (pos == null) return null;
        for (SupportCandidate candidate : candidates) if (sameCell(candidate.pos, pos)) return candidate;
        return null;
    }

    static boolean sameCell(int[] a, int[] b) {
        return a != null && b != null && a[0] == b[0] && a[1] == b[1] && a[2] == b[2];
    }

     


    private static Line optimalLine(Object p, Object world, float yawValue, float forward, float strafe)
        throws Exception {
        double[] direction = chooseDirection(inputAngle(yawValue, forward, strafe));
        double playerX = (double) InferedFastScaffoldBridge.positionX.invoke(p);
        double playerY = (double) InferedFastScaffoldBridge.positionY.invoke(p);
        double playerZ = (double) InferedFastScaffoldBridge.positionZ.invoke(p);
        SupportCandidate support = supportReference(p, world);
        if (support == null) return null;

        Line placed = fittedPlacedLine();
        double anchorX, anchorZ;
        if (placed != null && placed.dx * direction[0] + placed.dz * direction[1] >= .5) {
            double[] nearest = placed.nearest(playerX, playerY, playerZ);
            anchorX = nearest[0];
            anchorZ = nearest[2];
        } else {
            anchorX = playerX;
            anchorZ = playerZ;
        }
        return new Line(anchorX, playerY, anchorZ, direction[0], direction[1]);
    }

    private static Line fittedPlacedLine() {
        if (lastPlacedBlocks.size() < 2) return null;
        int[] last = null, previous = null;
        for (int[] cell : lastPlacedBlocks) {
            previous = last;
            last = cell;
        }
        if (last == null || previous == null) return null;
        double ax = previous[0] + .5, az = previous[2] + .5;
        double bx = last[0] + .5, bz = last[2] + .5;
        double dx = bx - ax, dz = bz - az, length = Math.hypot(dx, dz);
        if (length < 1e-6) return null;
        return new Line((ax + bx) * .5, 0, (az + bz) * .5, dx / length, dz / length);
    }

    private static void trackPlacedBlock(int[] cell) {
        if (!lastPlacedBlocks.isEmpty() && sameCell(lastPlacedBlocks.peekLast(), cell)) return;
        while (lastPlacedBlocks.size() >= MAX_LAST_PLACED) lastPlacedBlocks.removeFirst();
        lastPlacedBlocks.addLast(cell);
    }

     

     
    private static double[] fallOffPosition(Object p, Object world, Line line) throws Exception {
        double playerX = (double) InferedFastScaffoldBridge.positionX.invoke(p);
        double playerY = (double) InferedFastScaffoldBridge.positionY.invoke(p);
        double playerZ = (double) InferedFastScaffoldBridge.positionZ.invoke(p);
        double[] nearest = line.nearest(playerX, playerY, playerZ);
        int cellY = (int) Math.floor(playerY + .001) - 1;
        int previousX = 0, previousZ = 0;
        boolean sampled = false;
        for (double step = .1; step <= 3.0; step += .05) {
            double x = nearest[0] + line.dx * step, z = nearest[2] + line.dz * step;
            int cellX = (int) Math.floor(x), cellZ = (int) Math.floor(z);
            if (sampled && cellX == previousX && cellZ == previousZ) continue;
            if (isEmpty(world, cellX, cellY, cellZ)) return new double[] {x, playerY, z};
            previousX = cellX;
            previousZ = cellZ;
            sampled = true;
        }
        return null;
    }

    private static boolean closeToEdge(Object p, Object world) throws Exception {
        Object box = InferedFastScaffoldBridge.boundingBox.invoke(p);
        double minX = (double) InferedFastScaffoldBridge.boxMinX.get(box);
        double minZ = (double) InferedFastScaffoldBridge.boxMinZ.get(box);
        double maxX = (double) InferedFastScaffoldBridge.boxMaxX.get(box);
        double maxZ = (double) InferedFastScaffoldBridge.boxMaxZ.get(box);
        int y = (int) Math.floor((double) InferedFastScaffoldBridge.boxMinY.get(box)) - 1;
        for (double x : new double[] {minX + PREDICTION_CUTOFF, maxX - PREDICTION_CUTOFF}) {
            for (double z : new double[] {minZ + PREDICTION_CUTOFF, maxZ - PREDICTION_CUTOFF}) {
                if (isEmpty(world, (int) Math.floor(x), y, (int) Math.floor(z))) return true;
            }
        }
        return false;
    }

     
    private static double[] rotate(double x, double z, double angle) {
        double cos = Math.cos(angle), sin = Math.sin(angle);
        return new double[] {x * cos - z * sin, x * sin + z * cos};
    }

     



    private static double[] predictedPlacement(Object p, Object world, Line line) throws Exception {
        if (line == null) return null;
         
        if (closeToEdge(p, world)) return null;
        double[] fallOff = fallOffPosition(p, world, line);
        if (fallOff == null) return null;
        double playerX = (double) InferedFastScaffoldBridge.positionX.invoke(p);
        double playerZ = (double) InferedFastScaffoldBridge.positionZ.invoke(p);
        double dx = fallOff[0] - playerX, dz = fallOff[2] - playerZ;
        double length = Math.hypot(dx, dz);
        double back = length < 1e-6 || BOOTSTRAP_BACKOFF <= 0 ? 0 : BOOTSTRAP_BACKOFF / length;
        double[] bootstrap = {fallOff[0] - dx * back, fallOff[1], fallOff[2] - dz * back};
        double[] average = averagePlacementOffset();
        if (average == null) return bootstrap;
        double angle = Math.atan2(line.dz, line.dx);
        double[] offset = rotate(average[0], average[1], -angle);
        double[] predicted = {fallOff[0] + offset[0], fallOff[1], fallOff[2] + offset[1]};
        double blend = WARMUP_PLACEMENTS <= 0 ? 1.0
            : Math.min(1.0, (double) placementOffsets.size() / WARMUP_PLACEMENTS);
        return new double[] {
            bootstrap[0] + (predicted[0] - bootstrap[0]) * blend,
            bootstrap[1],
            bootstrap[2] + (predicted[2] - bootstrap[2]) * blend
        };
    }

    private static double[] averagePlacementOffset() {
        if (placementOffsets.isEmpty()) return null;
        double x = 0, z = 0;
        for (double[] offset : placementOffsets) {
            x += offset[0];
            z += offset[1];
        }
        return new double[] {x / placementOffsets.size(), z / placementOffsets.size()};
    }

    private static void recordPlacement(Object p, Line line, double[] fallOff) throws Exception {
        if (line == null || fallOff == null) return;
        double dx = (double) InferedFastScaffoldBridge.positionX.invoke(p) - fallOff[0];
        double dz = (double) InferedFastScaffoldBridge.positionZ.invoke(p) - fallOff[2];
        double[] rotated = rotate(dx, dz, Math.atan2(line.dz, line.dx));
        placementOffsets.addLast(rotated);
        while (placementOffsets.size() > MAX_PLACEMENT_OFFSETS) placementOffsets.removeFirst();
    }

     

     


    private static void towerMotion(Object p, Object world, boolean jumpHeld) throws Exception {
        if (!jumpHeld) {
            towerJumpOffY = Double.NaN;
            return;
        }
        double y = (double) InferedFastScaffoldBridge.positionY.invoke(p);
        if (Double.isNaN(towerJumpOffY)) {
            if ((boolean) InferedFastScaffoldBridge.grounded.invoke(p)) towerJumpOffY = y;
            return;
        }
        if (!hasSupport(p, world) || !validStack(p, InferedFastScaffoldBridge.held.invoke(p))) {
            towerJumpOffY = Double.NaN;
            return;
        }
        if (y <= towerJumpOffY + .78) return;
        InferedFastScaffoldBridge.setPosition.invoke(p, (double) InferedFastScaffoldBridge.positionX.invoke(p),
            Math.floor(y), (double) InferedFastScaffoldBridge.positionZ.invoke(p));
        Object motion = InferedFastScaffoldBridge.deltaMovement.invoke(p);
        double mx = (double) InferedFastScaffoldBridge.vx.invoke(motion);
        double mz = (double) InferedFastScaffoldBridge.vz.invoke(motion);
        InferedFastScaffoldBridge.setDeltaMovement.invoke(p, InferedFastScaffoldBridge.vecConstructor.newInstance(mx, .42, mz));
        towerJumpOffY = y;
    }

     

     

    static int[][] normalOffsets() {
        int[][] offsets = new int[NORMAL_OFFSETS.length][];
        for (int i = 0; i < offsets.length; i++) offsets[i] = NORMAL_OFFSETS[i].clone();
        return offsets;
    }

    private static int[][] buildNormalOffsets() {
        List<int[]> offsets = new ArrayList<>(18);
        for (int x = -1; x <= 1; x++) {
            for (int z = -1; z <= 1; z++) {
                offsets.add(new int[] {x, 0, z});
                offsets.add(new int[] {x, -1, z});
            }
        }
        offsets.sort(Comparator.<int[]>comparingInt(o -> o[0] * o[0] + o[1] * o[1] + o[2] * o[2])
            .thenComparingInt(o -> o[1])
            .thenComparingInt(o -> o[0])
            .thenComparingInt(o -> o[2]));
        return offsets.toArray(int[][]::new);
    }

     

    private static int[][] normalCandidates(double px, double py, double pz, Line line) {
        int baseX = (int) Math.floor(px), baseZ = (int) Math.floor(pz);
        int baseY = targetedY(py);
         
         
        List<int[]> offsets = new ArrayList<>(NORMAL_OFFSETS.length);
        for (int[] offset : NORMAL_OFFSETS) {
            if (offset[1] == 0 || falling) offsets.add(offset);
        }
        int[][] cells = new int[offsets.size()][];
        for (int i = 0; i < cells.length; i++) {
            int[] offset = offsets.get(i);
            cells[i] = new int[] {baseX + offset[0], baseY + offset[1], baseZ + offset[2]};
        }
        Arrays.sort(cells, (a, b) -> {
            double distanceA = line != null ? line.distanceSqr(a[0] + .5, a[2] + .5)
                : square(a[0] + .5 - px) + square(a[2] + .5 - pz);
            double distanceB = line != null ? line.distanceSqr(b[0] + .5, b[2] + .5)
                : square(b[0] + .5 - px) + square(b[2] + .5 - pz);
            return Double.compare(distanceA, distanceB);
        });
        return cells;
    }

     

    private static int[][] expandCandidates(double px, double py, double pz, float yawValue, int length) {
        int baseX = (int) Math.floor(px), baseZ = (int) Math.floor(pz);
        int baseY = targetedY(py);
        double radians = Math.toRadians(yawValue);
        int stepX = (int) Math.round(-Math.sin(radians)), stepZ = (int) Math.round(Math.cos(radians));
        List<int[]> cells = new ArrayList<>(length + 1);
        for (int i = 0; i <= length; i++) {
            int[] cell = {baseX + stepX * i, baseY, baseZ + stepZ * i};
            if (!containsCell(cells, cell)) cells.add(cell);
        }
        return cells.toArray(int[][]::new);
    }

     

    private static int targetedY(double py) {
        return (int) Math.floor(py + .001) - 1;
    }

    private static boolean containsCell(List<int[]> cells, int[] cell) {
        for (int[] existing : cells) if (sameCell(existing, cell)) return true;
        return false;
    }

    private static double square(double value) {
        return value * value;
    }

     

    private static boolean needsSupport(Object world, double px, double py, double pz) throws Exception {
        int cellX = (int) Math.floor(px), cellZ = (int) Math.floor(pz);
        int baseY = targetedY(py);
        if (falling) return isEmpty(world, cellX, baseY, cellZ) || isEmpty(world, cellX, baseY - 1, cellZ);
        return isEmpty(world, cellX, baseY, cellZ);
    }

    private static boolean isEmpty(Object world, int x, int y, int z) throws Exception {
        Object pos = InferedFastScaffoldBridge.blockPos.newInstance(x, y, z);
        Object state = InferedFastScaffoldBridge.air.invoke(world, pos);
        Object shape = InferedFastScaffoldBridge.collisionShape.invoke(state, world, pos);
        return Boolean.TRUE.equals(InferedFastScaffoldBridge.shapeEmpty.invoke(shape));
    }

    private static boolean isAir(Object world, int[] xyz) throws Exception {
        Object state = InferedFastScaffoldBridge.air.invoke(world,
            InferedFastScaffoldBridge.blockPos.newInstance(xyz[0], xyz[1], xyz[2]));
        if (state.getClass() != airStateType) {
            airStateAccess = state.getClass().getMethod("method_26215");
            airStateType = state.getClass();
        }
        return (boolean) airStateAccess.invoke(state);
    }

    private static Method[] positionAccessors(Object pos) throws Exception {
        if (pos.getClass() != positionType) {
            positionAccess = resolveAccessors(pos.getClass(), POSITION_METHODS);
            positionType = pos.getClass();
        }
        return positionAccess;
    }

    private static Method[] faceAccessors(Object face) throws Exception {
        if (face.getClass() != faceType) {
            faceAccess = resolveAccessors(face.getClass(), FACE_METHODS);
            faceType = face.getClass();
        }
        return faceAccess;
    }

    private static Method[] resolveAccessors(Class<?> type, String[] names) throws Exception {
        Method[] access = new Method[names.length];
        for (int i = 0; i < names.length; i++) access[i] = type.getMethod(names[i]);
        return access;
    }

     

    static final class FaceTarget {
        final int[] destination;
        final float yaw, pitch;
        final double turn;

        FaceTarget(int[] destination, float yaw, float pitch, double turn) {
            this.destination = destination;
            this.yaw = yaw;
            this.pitch = pitch;
            this.turn = turn;
        }
    }

    private static final int[][] FACES = {{0, 0, 1}, {0, 0, -1}, {1, 0, 0}, {-1, 0, 0}, {0, -1, 0}, {0, 1, 0}};

     
    private static int[][] stickyFirst(int[][] candidates) {
        if (lockedDestination == null) return candidates;
        int[][] ordered = new int[candidates.length][];
        int n = 0;
        for (int[] c : candidates) {
            if (sameCell(c, lockedDestination)) ordered[n++] = c;
        }
        for (int[] c : candidates) {
            if (!sameCell(c, lockedDestination)) ordered[n++] = c;
        }
        return ordered;
    }

     


    private static FaceTarget findBestTarget(Object p, Object world, int[][] candidates, double range,
                                             float fromYaw, float fromPitch, Object stack) throws Exception {
        Object eyes = InferedFastScaffoldBridge.eye.invoke(p);
        double ex = (double) InferedFastScaffoldBridge.vx.invoke(eyes);
        double ey = (double) InferedFastScaffoldBridge.vy.invoke(eyes);
        double ez = (double) InferedFastScaffoldBridge.vz.invoke(eyes);
        for (int[] cell : stickyFirst(candidates)) {
            if (!isAir(world, cell)) continue;
            FaceTarget best = null;
            for (int[] face : FACES) {
                int[] anchor = {cell[0] + face[0], cell[1] + face[1], cell[2] + face[2]};
                if (isAir(world, anchor)) continue;
                if (!faceFacesEye(ex, ey, ez, anchor, face)) continue;
                double tx = cell[0] + .5 + face[0] * .5;
                double ty = face[1] == -1 ? cell[1] : cell[1] + .5 + face[1] * .5;
                double tz = cell[2] + .5 + face[2] * .5;
                float[] angles = InferedFastScaffoldBridge.aim(ex, ey, ez, tx, ty, tz, range);
                if (angles == null) continue;
                double turn = InferedFastScaffoldBridge.angularTurn(fromYaw, fromPitch, angles[0], angles[1]);
                if (best != null && turn >= best.turn) continue;
                Object hit = vanillaRaycast(p, range, angles[0], angles[1]);
                if (matches(hit, cell) && canPlaceAt(p, hit, cell, stack)) {
                    best = new FaceTarget(cell, angles[0], angles[1], turn);
                }
            }
            if (best != null) {
                lockedDestination = best.destination;
                return best;
            }
        }
        return null;
    }

     

    static boolean faceFacesEye(double ex, double ey, double ez, int[] anchor, int[] offset) {
        final double margin = .05;
        int ax = anchor[0], ay = anchor[1], az = anchor[2];
        if (offset[0] == 1) return ex < ax - margin;
        if (offset[0] == -1) return ex > ax + 1 + margin;
        if (offset[2] == 1) return ez < az - margin;
        if (offset[2] == -1) return ez > az + 1 + margin;
        if (offset[1] == -1) return ey > ay + 1 + margin;
        return false;
    }

     

    private static void place(Object p, Object hit, int[] target, boolean autoBlock, int delay) throws Exception {
        Object stack = selectStack(p, true, autoBlock);
        if (stack == null || InferedFastScaffoldBridge.cooldownField.getInt(InferedFastScaffoldBridge.client) > 0
            || !matches(hit, target) || !isAir(InferedFastScaffoldBridge.worldField.get(InferedFastScaffoldBridge.client), target)
            || !canPlaceAt(p, hit, target, stack)) {
            restoreSlot();
            clearRotationState();
            return;
        }
        Object world = InferedFastScaffoldBridge.worldField.get(InferedFastScaffoldBridge.client);
        Line line = optimalLine(p, world, (float) InferedFastScaffoldBridge.yaw.invoke(p),
            (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.forwardKey) ? 1 : 0)
                - (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.backKey) ? 1 : 0),
            (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.leftKey) ? 1 : 0)
                - (InferedFastScaffoldBridge.down(InferedFastScaffoldBridge.rightKey) ? 1 : 0));
        double[] fallOff = line == null ? null : fallOffPosition(p, world, line);
        Object oldPick = InferedFastScaffoldBridge.pickField.get(InferedFastScaffoldBridge.client);
        Object oldTarget = InferedFastScaffoldBridge.targetField.get(InferedFastScaffoldBridge.client);
        if (tickShadow && !armBody()) {
            restoreSlot();
            clearRotationState();
            return;
        }
        phase = PLACE;
        try {
            InferedFastScaffoldBridge.pickField.set(InferedFastScaffoldBridge.client, hit);
            InferedFastScaffoldBridge.targetField.set(InferedFastScaffoldBridge.client, null);
             
             
            InferedFastScaffoldBridge.use.invoke(InferedFastScaffoldBridge.client);
             
             
            int cooldown = InferedFastScaffoldBridge.cooldownField.getInt(InferedFastScaffoldBridge.client);
            if (cooldown > PLACEMENT_INTERVAL) InferedFastScaffoldBridge.cooldownField.setInt(
                InferedFastScaffoldBridge.client, PLACEMENT_INTERVAL);
            nextUseTick = inputTick + PLACEMENT_INTERVAL + delay;
            placeHoldTick = inputTick;
            trackPlacedBlock(target);
            recordPlacement(p, line, fallOff);
        } finally {
            InferedFastScaffoldBridge.pickField.set(InferedFastScaffoldBridge.client, oldPick);
            InferedFastScaffoldBridge.targetField.set(InferedFastScaffoldBridge.client, oldTarget);
            restoreSlot();
            clearRotationState();
        }
    }

    private static Object vanillaRaycast(Object p, double range, float yawValue, float pitchValue) throws Exception {
        Object start = InferedFastScaffoldBridge.eye.invoke(p);
        double sx = (double) InferedFastScaffoldBridge.vx.invoke(start);
        double sy = (double) InferedFastScaffoldBridge.vy.invoke(start);
        double sz = (double) InferedFastScaffoldBridge.vz.invoke(start);
        double yawRadians = Math.toRadians(yawValue), pitchRadians = Math.toRadians(pitchValue);
        double scale = Math.cos(pitchRadians);
        double dx = -Math.sin(yawRadians) * scale, dy = -Math.sin(pitchRadians), dz = Math.cos(yawRadians) * scale;
        Object end = InferedFastScaffoldBridge.vecConstructor.newInstance(sx + dx * range, sy + dy * range, sz + dz * range);
        Object context = InferedFastScaffoldBridge.rayContextConstructor.newInstance(
            start, end, InferedFastScaffoldBridge.shapeType, InferedFastScaffoldBridge.fluidMode, p);
        return InferedFastScaffoldBridge.raycast.invoke(InferedFastScaffoldBridge.worldField.get(InferedFastScaffoldBridge.client),
            context);
    }

    private static int[] matchingTarget(Object hit, int[][] candidates, Object world) throws Exception {
        int[] destination = hitDestination(hit);
        if (destination == null) return null;
        for (int[] cell : candidates) if (sameCell(cell, destination) && isAir(world, cell)) return cell;
        return null;
    }

    private static int[] hitDestination(Object hit) throws Exception {
        if (!InferedFastScaffoldBridge.blockHit.isInstance(hit)
            || !((Enum<?>) InferedFastScaffoldBridge.hitKind.invoke(hit)).name().equals("BLOCK")) return null;
        Object block = InferedFastScaffoldBridge.hitBlock.invoke(hit), face = InferedFastScaffoldBridge.hitFace.invoke(hit);
        Method[] coord = positionAccessors(block), step = faceAccessors(face);
        int[] destination = new int[3];
        for (int axis = 0; axis < 3; axis++) {
            destination[axis] = (int) coord[axis].invoke(block) + (int) step[axis].invoke(face);
        }
        return destination;
    }

    private static boolean matches(Object hit, int[] destination) throws Exception {
        return destination != null && sameCell(hitDestination(hit), destination);
    }

     



    private static boolean validCrosshairTarget(Object p, Object hit, float minDist) throws Exception {
        if (minDist <= 0) return true;
        if (!InferedFastScaffoldBridge.blockHit.isInstance(hit)
            || !((Enum<?>) InferedFastScaffoldBridge.hitKind.invoke(hit)).name().equals("BLOCK")) return false;
        Object block = InferedFastScaffoldBridge.hitBlock.invoke(hit);
        Object face = InferedFastScaffoldBridge.hitFace.invoke(hit);
        Method[] coord = positionAccessors(block), step = faceAccessors(face);
        Object eyes = InferedFastScaffoldBridge.eye.invoke(p);
        double ex = (double) InferedFastScaffoldBridge.vx.invoke(eyes);
        double ez = (double) InferedFastScaffoldBridge.vz.invoke(eyes);
        for (int axis = 0; axis < 3; axis++) {
            int delta = (int) step[axis].invoke(face);
            if (delta == 0) continue;
             
             
            if (axis == 1) return true;
            int base = (int) coord[axis].invoke(block);
            double eyeCoordinate = axis == 0 ? ex : ez;
            return Math.abs(eyeCoordinate - (base + (delta > 0 ? 1 : 0))) >= minDist;
        }
        return true;
    }

     

    private static boolean hasSupport(Object p, Object world) throws Exception {
        double x = (double) InferedFastScaffoldBridge.positionX.invoke(p);
        double y = (double) InferedFastScaffoldBridge.positionY.invoke(p);
        double z = (double) InferedFastScaffoldBridge.positionZ.invoke(p);
        for (double ox : new double[] {-.29, 0, .29}) {
            for (double oz : new double[] {-.29, 0, .29}) {
                Object pos = InferedFastScaffoldBridge.blockPos.newInstance(
                    (int) Math.floor(x + ox), (int) Math.floor(y - .01), (int) Math.floor(z + oz));
                Object state = InferedFastScaffoldBridge.air.invoke(world, pos);
                if ((boolean) InferedFastScaffoldBridge.fullCube.invoke(state, world, pos)) return true;
            }
        }
        return false;
    }

     


    private static boolean validStack(Object p, Object stack) throws Exception {
        if (stack == null || (boolean) InferedFastScaffoldBridge.empty.invoke(stack)) return false;
        Object value = InferedFastScaffoldBridge.item.invoke(stack);
        if (!InferedFastScaffoldBridge.blockItemClass.isInstance(value)) return false;
        Object block = InferedFastScaffoldBridge.getBlock.invoke(value);
        if (InferedFastScaffoldBridge.fallingBlock.isInstance(block)) return false;
        String key = (String) InferedFastScaffoldBridge.blockKey.invoke(block);
        if (DISALLOWED.contains(key)) return false;
        Object pos = InferedFastScaffoldBridge.blockPos.newInstance(
            (int) Math.floor((double) InferedFastScaffoldBridge.positionX.invoke(p)), supportY,
            (int) Math.floor((double) InferedFastScaffoldBridge.positionZ.invoke(p)));
        return (boolean) InferedFastScaffoldBridge.fullCube.invoke(
            InferedFastScaffoldBridge.defaultState.invoke(block), InferedFastScaffoldBridge.worldField.get(
                InferedFastScaffoldBridge.client), pos);
    }

     

    private static final Set<String> DISALLOWED = Set.of(
        "Block{minecraft:tnt}", "Block{minecraft:cobweb}", "Block{minecraft:nether_portal}",
        "Block{minecraft:magma_block}", "Block{minecraft:slime_block}", "Block{minecraft:honey_block}",
        "Block{minecraft:soul_sand}", "Block{minecraft:ice}", "Block{minecraft:packed_ice}",
        "Block{minecraft:blue_ice}");

     

    private static final Set<String> UNFAVOURABLE = Set.of(
        "Block{minecraft:crafting_table}", "Block{minecraft:jigsaw}", "Block{minecraft:smithing_table}",
        "Block{minecraft:fletching_table}", "Block{minecraft:enchanting_table}",
        "Block{minecraft:cauldron}", "Block{minecraft:magma_block}");

     

    private static int preference(Object stack) throws Exception {
        Object value = InferedFastScaffoldBridge.item.invoke(stack);
        Object block = InferedFastScaffoldBridge.getBlock.invoke(value);
        String key = (String) InferedFastScaffoldBridge.blockKey.invoke(block);
        int score = 0;
        if (!UNFAVOURABLE.contains(key)) score += 100;
        Object pos = InferedFastScaffoldBridge.blockPos.newInstance(0, 0, 0);
        if ((boolean) InferedFastScaffoldBridge.fullCube.invoke(InferedFastScaffoldBridge.defaultState.invoke(block),
            InferedFastScaffoldBridge.worldField.get(InferedFastScaffoldBridge.client), pos)) score += 50;
        return score;
    }

    private static Object selectStack(Object p, boolean select, boolean autoBlock) throws Exception {
        Object stack = InferedFastScaffoldBridge.held.invoke(p);
        if (validStack(p, stack)) return stack;
        if (!autoBlock) return null;
        Object inv = InferedFastScaffoldBridge.inventory.invoke(p);
        Object bestStack = null;
        int best = -1, bestScore = Integer.MIN_VALUE, bestCount = -1;
        for (int slot = 0; slot < 9; slot++) {
            Object candidate = InferedFastScaffoldBridge.inventoryStack.invoke(inv, slot);
            if (!validStack(p, candidate)) continue;
            int score = preference(candidate);
            int count = (int) InferedFastScaffoldBridge.stackCount.invoke(candidate);
            if (score > bestScore || (score == bestScore && count > bestCount)) {
                best = slot;
                bestScore = score;
                bestCount = count;
                bestStack = candidate;
            }
        }
        if (best < 0) return null;
        if (select) {
            slotInventory = inv;
            slotPlayer = p;
            originalSlot = InferedFastScaffoldBridge.selectedSlot.getInt(inv);
            assistedSlot = best;
            InferedFastScaffoldBridge.selectedSlot.setInt(inv, best);
        }
        return bestStack;
    }

    private static void restoreSlot() throws Exception {
        if (slotInventory != null && InferedFastScaffoldBridge.selectedSlot.getInt(slotInventory) == assistedSlot) {
            InferedFastScaffoldBridge.selectedSlot.setInt(slotInventory, originalSlot);
        }
        slotInventory = slotPlayer = null;
        originalSlot = assistedSlot = -1;
    }

    private static boolean canPlaceAt(Object p, Object hit, int[] target, Object stack) throws Exception {
        if (stack == null || !matches(hit, target)) return false;
        Object value = InferedFastScaffoldBridge.item.invoke(stack);
        Object context = InferedFastScaffoldBridge.placementContext.newInstance(
            p, InferedFastScaffoldBridge.mainHand, stack, hit);
        context = InferedFastScaffoldBridge.updateContext.invoke(value, context);
        if (context == null) return false;
        Object pos = InferedFastScaffoldBridge.placementPos.invoke(context);
        Method[] coord = positionAccessors(pos);
        if ((int) coord[0].invoke(pos) != target[0]
            || (int) coord[1].invoke(pos) != target[1]
            || (int) coord[2].invoke(pos) != target[2]) return false;
        Object state = InferedFastScaffoldBridge.placementState.invoke(value, context);
        return state != null && (boolean) InferedFastScaffoldBridge.canPlace.invoke(value, context, state);
    }

     
    static int[][] targets(double px, double py, double pz) {
        return normalCandidates(px, py, pz, null);
    }

     
    static int[][] expandTargets(double px, double py, double pz, float yawValue, int length) {
        return expandCandidates(px, py, pz, yawValue, length);
    }
}
