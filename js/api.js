import { invoke as tauriInvoke } from '@tauri-apps/api/core';

// Check if running in Tauri environment
const isTauri = () => !!window.__TAURI_INTERNALS__;

// ── Web Mock Logic ──
const mockInvoke = async (cmd, args = {}) => {
  console.warn(`[Web Mode] Mocking command: ${cmd}`, args);
  
  // Artificial delay for realism
  await new Promise(r => setTimeout(r, 400));

  switch (cmd) {
    case 'get_system_info':
      return {
        os_info: "Windows 11 Pro (Web Demo)",
        os_name: "Windows 11 Pro",
        os_version: "(Web Demo)",
        hostname: "DEMO-PC",
        cpu_info: "Intel Core i9-14900K",
        cpu_name: "Intel Core i9-14900K",
        cpu_usage: 12.5 + Math.random() * 5,
        gpu_info: "NVIDIA GeForce RTX 4090",
        ram_total_gb: 64.0,
        total_ram_mb: 65536,
        used_ram_mb: 8400 + Math.round(Math.random() * 200),
        disks: [
          { mount_point: "C:", total_gb: 512, free_gb: 245.3 },
          { mount_point: "D:", total_gb: 1024, free_gb: 780.1 }
        ]
      };
    case 'get_realtime_stats':
      return {
        cpu_usage: 12.5 + Math.random() * 5,
        ram_used_mb: 8400 + Math.random() * 200,
        gpu_usage: 5.0 + Math.random() * 2,
        cpu_temp_c: 42 + Math.random() * 8
      };
    case 'get_registry_status':
      return [
        { name: "Disable Nagle Algorithm", applied: false },
        { name: "Game Priority Boost", applied: false },
        { name: "Disable Full-Screen Optimizations", applied: true },
        { name: "Disable Game Bar / DVR", applied: false }
      ];
    case 'validate_license_remote':
      return { success: false, message: "License validation requires the desktop app. Download RustOpti to activate." };
    case 'check_license_status':
      return true; // Web demo mode — always unlocked
    case 'get_license_cache_status':
      return 'valid';
    case 'revalidate_license':
      return { status: 'valid', message: 'Demo mode' };
    case 'revoke_license':
      return null;
    case 'get_license_info':
      return { active: true, plan: 'monthly', expires_at: '2025-12-31 23:59:59' };
    case 'get_hwid':
      return "WEB-DEMO-HEX-ID-0123456789";
    case 'get_power_plans':
      return [
        { name: "Balanced", active: false },
        { name: "High Performance", active: true },
        { name: "RustOpti Ultimate", active: false }
      ];
    case 'get_cleanup_preview':
      return [
        { category: "Temp Files", size_mb: 540.2 },
        { category: "Browser Cache", size_mb: 380.8 },
        { category: "System Logs", size_mb: 319.5 }
      ];
    case 'list_backups':
      return ["manual_backup_2026.reg", "auto_registry_pre_opt.reg"];
    case 'get_network_status':
      return [
        { name: "TCP Auto-Tuning", description: "Optimize TCP window size", optimized: false },
        { name: "Nagle Algorithm", description: "Disable packet batching for lower latency", optimized: false },
        { name: "Network Throttling", description: "Remove OS network speed limit", optimized: true }
      ];
    case 'detect_gpu_vendor':
      return "NVIDIA GeForce RTX 4090 (Demo)";
    case 'detect_rust_installation':
      return "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Rust";
    case 'get_recommended_launch_options':
      return "-high -maxMem=16384 -malloc=system -force-feature-level-11-0 -cpuCount=8 -exThreads=16";
    case 'get_recommended_console_commands':
      return ["fps.limit 0", "gc.buffer 2048", "physics.steps 60", "batching.colliders 1"];
    case 'get_standby_info':
      return { total_ram_mb: 65536, used_ram_mb: 24000, standby_mb: 8200, free_ram_mb: 33336, usage_percent: 36.6 };
    case 'get_islc_status':
      return { monitor_running: false, threshold_mb: 1024, total_clears: 0 };
    case 'get_core_parking_status':
      return { total_cores: 24, min_cores_percent: 100, cores_parked: false };
    case 'apply_registry_tweaks':
    case 'apply_gpu_tweaks':
    case 'apply_power_tweaks':
    case 'apply_network_tweaks':
    case 'apply_rust_tweaks':
    case 'unpark_all_cores':
    case 'repark_cores':
    case 'backup_all_before_optimization':
      return [{ success: true, name: cmd, message: `[Demo] ${cmd} applied successfully!` }];
    case 'kill_bloatware':
      return ["[Demo] Closed 3 unnecessary background processes"];
    case 'run_disk_cleanup':
      return [{ success: true, message: "[Demo] Cleaned 1.2 GB of temporary files" }];
    case 'get_defender_status':
      return { name: 'Defender', success: false, message: '⚠ Rust folder NOT excluded' };
    case 'add_defender_exclusion':
      return { name: 'Defender', success: true, message: '[Demo] Exclusion added' };
    case 'remove_defender_exclusion':
      return { name: 'Defender', success: true, message: '[Demo] Exclusion removed' };
    case 'get_large_pages_status':
      return { name: 'Large Pages', success: false, message: '⚠ Large Pages disabled' };
    case 'enable_large_pages':
      return { name: 'Large Pages', success: true, message: '[Demo] Large Pages enabled' };
    case 'get_game_boost_status':
      return { defender_excluded: false, large_pages_enabled: false, game_mode_active: false, rust_running: false, rust_pid: null, rust_affinity: 'Default' };
    case 'activate_game_mode':
      return [{ name: 'Game Mode', success: true, message: '[Demo] Game Mode activated' }];
    case 'deactivate_game_mode':
      return { name: 'Game Mode', success: true, message: '[Demo] Game Mode deactivated' };
    case 'get_msi_mode_status':
      return [{ name: 'MSI Mode: NVIDIA GeForce RTX 4090', optimized: false, current_value: 'Disabled' }];
    case 'enable_msi_mode':
      return [{ name: 'MSI Mode', success: true, message: '[Demo] MSI Mode enabled' }];
    case 'get_sysmain_status':
      return { name: 'SysMain (Superfetch)', optimized: false, current_value: 'Running' };
    case 'disable_sysmain':
      return { name: 'SysMain', success: true, message: '[Demo] SysMain disabled' };
    case 'enable_sysmain':
      return { name: 'SysMain', success: true, message: '[Demo] SysMain enabled' };
    case 'get_visual_effects_status':
      return { name: 'Visual Effects', optimized: false, current_value: 'Let Windows decide' };
    case 'disable_visual_effects':
      return [{ name: 'Visual Effects', success: true, message: '[Demo] Visual effects disabled' }];
    case 'restore_visual_effects':
      return { name: 'Visual Effects', success: true, message: '[Demo] Visual effects restored' };
    case 'get_timer_status':
      return { current_resolution_ms: 15.625, timer_boosted: false, hpet_enabled: true };
    case 'boost_timer_resolution':
      return { name: 'Timer Resolution', success: true, message: '[Demo] Timer set to 0.500ms' };
    case 'reset_timer_resolution':
      return { name: 'Timer Resolution', success: true, message: '[Demo] Timer reset to 15.625ms' };
    case 'disable_hpet':
      return { name: 'HPET Disable', success: true, message: '[Demo] HPET disabled' };
    case 'enable_hpet':
      return { name: 'HPET Enable', success: true, message: '[Demo] HPET restored' };
    case 'clear_standby_now':
      return { success: true, message: "[Demo] Standby list cleared" };
    case 'start_islc_monitor':
      return { success: true, message: `[Demo] ISLC Monitor started (threshold: ${args.thresholdMb} MB)` };
    case 'stop_islc_monitor':
      return { success: true, message: "[Demo] ISLC Monitor stopped" };
    case 'create_restore_point':
      return `[Demo] Restore point created: ${args.description}`;
    default:
      return { success: true, message: `[Demo] ${cmd} applied successfully!` };
  }
};

const invoke = (cmd, args = {}) => {
  if (isTauri()) {
    return tauriInvoke(cmd, args);
  } else {
    return mockInvoke(cmd, args);
  }
};

// ── System Info ─────────────────────────────────────────────────
export const getSystemInfo = () => invoke('get_system_info');
export const getRealtimeStats = () => invoke('get_realtime_stats');

// ── Registry ────────────────────────────────────────────────────
export const getRegistryStatus = () => invoke('get_registry_status');
export const applyRegistryTweaks = () => invoke('apply_registry_tweaks');

// ── GPU ─────────────────────────────────────────────────────────
export const detectGpuVendor = () => invoke('detect_gpu_vendor');
export const applyGpuTweaks = () => invoke('apply_gpu_tweaks');

// ── Power ───────────────────────────────────────────────────────
export const getPowerPlans = () => invoke('get_power_plans');
export const applyPowerTweaks = () => invoke('apply_power_tweaks');

// ── Process ─────────────────────────────────────────────────────
export const getProcessList = () => {
    if (!isTauri()) return Promise.resolve([
        { pid: 1234, name: "RustClient.exe", memory_mb: 8400, cpu_usage: 65.2 },
        { pid: 5678, name: "Steam.exe", memory_mb: 450, cpu_usage: 1.5 },
        { pid: 9012, name: "Discord.exe", memory_mb: 620, cpu_usage: 2.1 }
    ]);
    return invoke('get_process_list');
};
export const killProcess = (pid) => invoke('kill_process', { pid });
export const killBloatware = () => invoke('kill_bloatware');
export const setProcessPriority = (pid, priority) => invoke('set_process_priority', { pid, priority });

// ── Network ─────────────────────────────────────────────────────
export const getNetworkStatus = () => invoke('get_network_status');
export const applyNetworkTweaks = () => invoke('apply_network_tweaks');

// ── Disk ────────────────────────────────────────────────────────
export const getCleanupPreview = () => invoke('get_cleanup_preview');
export const runDiskCleanup = () => invoke('run_disk_cleanup');

// ── RAM ─────────────────────────────────────────────────────────
export const getRamStatus = () => invoke('get_ram_status');
export const optimizeRam = () => invoke('optimize_ram');

// ── Startup ─────────────────────────────────────────────────────
export const getStartupItems = () => invoke('get_startup_items');
export const disableStartupItem = (name, location) => invoke('disable_startup_item', { name, location });
export const getDisableRecommendations = () => invoke('get_disable_recommendations');

// ── Rust Game ───────────────────────────────────────────────────
export const detectRustInstallation = () => invoke('detect_rust_installation');
export const getRecommendedLaunchOptions = () => invoke('get_recommended_launch_options');
export const getRecommendedConsoleCommands = () => invoke('get_recommended_console_commands');
export const applyRustTweaks = () => invoke('apply_rust_tweaks');

// ── Backup ──────────────────────────────────────────────────────
export const createRestorePoint = (description) => invoke('create_restore_point', { description });
export const exportRegistryBackup = (keys) => invoke('export_registry_backup', { keys });
export const restoreRegistryBackup = (filename) => invoke('restore_registry_backup', { filename });
export const listBackups = () => invoke('list_backups');
export const backupAllBeforeOptimization = () => invoke('backup_all_before_optimization');

// ── Configuration ───────────────────────────────────────────────
export const getConfig = () => invoke('get_config');
export const updateConfig = (new_config) => invoke('update_config', { newConfig: new_config });

// ── ISLC (Standby List Cleaner) ─────────────────────────────────
export const getStandbyInfo = () => invoke('get_standby_info');
export const clearStandbyNow = () => invoke('clear_standby_now');
export const startIslcMonitor = (thresholdMb) => invoke('start_islc_monitor', { thresholdMb });
export const stopIslcMonitor = () => invoke('stop_islc_monitor');
export const getIslcStatus = () => invoke('get_islc_status');

// ── Core Unparking ──────────────────────────────────────────────
export const getCoreParkingStatus = () => invoke('get_core_parking_status');
export const unparkAllCores = () => invoke('unpark_all_cores');
export const reparkCores = () => invoke('repark_cores');

// ── Game Boost (Defender, Large Pages, Game Mode) ──────────────
export const getDefenderStatus = () => invoke('get_defender_status');
export const addDefenderExclusion = () => invoke('add_defender_exclusion');
export const removeDefenderExclusion = () => invoke('remove_defender_exclusion');
export const getLargePagesStatus = () => invoke('get_large_pages_status');
export const enableLargePages = () => invoke('enable_large_pages');
export const getGameBoostStatus = () => invoke('get_game_boost_status');
export const activateGameMode = () => invoke('activate_game_mode');
export const deactivateGameMode = () => invoke('deactivate_game_mode');
export const subscriptionExpiredCleanup = () => invoke('subscription_expired_cleanup');
export const startActiveProtection = () => invoke('start_active_protection');
export const stopActiveProtection = () => invoke('stop_active_protection');
export const getActiveProtectionStatus = () => invoke('get_active_protection_status');

// ── System Tweaks (MSI, SysMain, Visual Effects) ───────────────
export const getMsiModeStatus = () => invoke('get_msi_mode_status');
export const enableMsiMode = () => invoke('enable_msi_mode');
export const getSysmainStatus = () => invoke('get_sysmain_status');
export const disableSysmain = () => invoke('disable_sysmain');
export const enableSysmain = () => invoke('enable_sysmain');
export const getVisualEffectsStatus = () => invoke('get_visual_effects_status');
export const disableVisualEffects = () => invoke('disable_visual_effects');
export const restoreVisualEffects = () => invoke('restore_visual_effects');

// ── Timer & HPET ───────────────────────────────────────────────
export const getTimerStatus = () => invoke('get_timer_status');
export const boostTimerResolution = () => invoke('boost_timer_resolution');
export const resetTimerResolution = () => invoke('reset_timer_resolution');
export const disableHpet = () => invoke('disable_hpet');
export const enableHpet = () => invoke('enable_hpet');

// ── Licensing ───────────────────────────────────────────────────
export const getHwid = () => invoke('get_hwid');

// Check license state from Rust memory (not localStorage!)
export const checkLicenseStatus = () => invoke('check_license_status');

// Get cache status: "valid" | "needs_recheck" | "expired"
export const getLicenseCacheStatus = () => invoke('get_license_cache_status');

// Revalidate license against server (periodic check, key stored in Rust memory)
export const revalidateLicense = () => invoke('revalidate_license');

// Revoke license in Rust memory (for "Reset License" button)
export const revokeLicense = () => invoke('revoke_license');

// Get subscription info (plan + expiry) for account page
export const getLicenseInfo = () => invoke('get_license_info');

export const validateLicenseKey = async (key) => {
  const hwid = await getHwid();

  try {
    const result = await invoke('validate_license_remote', { key, hwid });
    if (result.error) return { success: false, message: result.error };

    // Launch background service after successful activation
    if (result.success) {
      launchService().catch(() => {});
    }

    return result;
  } catch (e) {
    return { success: false, message: `System error: ${e}` };
  }
};

// Launch rustopti-service.exe in background (keeps timer resolution active)
export const launchService = () => invoke('launch_service');



export const getUpscalingStatus = () => invoke("get_upscaling_status");
export const setUpscaling = (enabled, sharpness) => invoke("set_upscaling", { enabled, sharpness });

