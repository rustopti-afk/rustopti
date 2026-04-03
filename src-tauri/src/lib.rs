pub mod commands;
pub mod utils;

use commands::*;
use utils::license_guard::LicenseState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Revert any tweaks left from a previous crash/kill
    utils::cleanup::revert_leftover_tweaks();

    // Security checks before launching UI
    utils::security::perform_all_security_checks();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // License state lives in Rust memory — frontend cannot tamper with it
        .manage(LicenseState::new())
        .invoke_handler(tauri::generate_handler![
            // System Info
            system_info::get_system_info,
            system_info::get_realtime_stats,
            // Registry
            registry::get_registry_status,
            registry::apply_registry_tweaks,
            // GPU
            gpu::detect_gpu_vendor,
            gpu::apply_gpu_tweaks,
            // Power
            power::get_power_plans,
            power::apply_power_tweaks,
            // Process
            process::get_process_list,
            process::kill_process,
            process::kill_bloatware,
            process::set_process_priority,
            // Network
            network::get_network_status,
            network::apply_network_tweaks,
            // Disk
            disk::get_cleanup_preview,
            disk::run_disk_cleanup,
            // RAM
            ram::get_ram_status,
            ram::optimize_ram,
            // Startup
            startup::get_startup_items,
            startup::disable_startup_item,
            startup::get_disable_recommendations,
            // Rust Game
            rust_game::detect_rust_installation,
            rust_game::get_recommended_launch_options,
            rust_game::get_recommended_console_commands,
            rust_game::apply_rust_tweaks,
            // Backup
            backup::create_restore_point,
            backup::export_registry_backup,
            backup::restore_registry_backup,
            backup::list_backups,
            backup::backup_all_before_optimization,
            // Config
            config::get_config,
            config::update_config,
            // ISLC (Standby List Cleaner)
            islc::get_standby_info,
            islc::clear_standby_now,
            islc::start_islc_monitor,
            islc::stop_islc_monitor,
            islc::get_islc_status,
            // Core Unparking
            core_unpark::get_core_parking_status,
            core_unpark::unpark_all_cores,
            core_unpark::repark_cores,
            // Game Boost (Defender, Large Pages, Game Mode, Affinity)
            game_boost::subscription_expired_cleanup,
            game_boost::start_active_protection,
            game_boost::stop_active_protection,
            game_boost::get_active_protection_status,
            game_boost::get_defender_status,
            game_boost::add_defender_exclusion,
            game_boost::remove_defender_exclusion,
            game_boost::get_large_pages_status,
            game_boost::enable_large_pages,
            game_boost::get_game_boost_status,
            game_boost::activate_game_mode,
            game_boost::deactivate_game_mode,
            // System tweaks (MSI, SysMain, Visual Effects)
            system_tweaks::get_msi_mode_status,
            system_tweaks::enable_msi_mode,
            system_tweaks::get_sysmain_status,
            system_tweaks::disable_sysmain,
            system_tweaks::enable_sysmain,
            system_tweaks::get_visual_effects_status,
            system_tweaks::disable_visual_effects,
            system_tweaks::restore_visual_effects,
            // Timer & HPET tweaks
            timer_tweaks::get_timer_status,
            timer_tweaks::boost_timer_resolution,
            timer_tweaks::reset_timer_resolution,
            timer_tweaks::disable_hpet,
            timer_tweaks::enable_hpet,
            // Licensing Protection (server-side state)
            licensing::validate_license_remote,
            licensing::check_license_status,
            licensing::get_license_cache_status,
            licensing::revalidate_license,
            licensing::revoke_license,
            licensing::get_license_info,
            // HWID
            utils::hwid::get_hwid,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                utils::cleanup::revert_all_tweaks();
            }
        });
}
