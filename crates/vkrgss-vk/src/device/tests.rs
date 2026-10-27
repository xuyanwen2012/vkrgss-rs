use super::*;

/// Scenario "Decision by driver".
#[test]
fn undrawn_images_are_not_released_on_a_headless_surface_of_an_old_mesa() {
    let mesa = |major, minor, patch| {
        (
            format!("llvmpipe Mesa {major}.{minor}.{patch} (LLVM 20.1.2)"),
            vk::make_api_version(0, major, minor, patch),
        )
    };
    for (version, expected) in [
        ((25, 2, 8), false),
        ((25, 3, 3), false),
        ((25, 3, 4), true),
        ((26, 2, 4), true),
    ] {
        let (driver, packed) = mesa(version.0, version.1, version.2);
        assert_eq!(
            releases_undrawn_images(true, &driver, packed),
            expected,
            "{driver}"
        );
    }
    // Another driver encodes its version in its own way; the number says nothing.
    assert!(releases_undrawn_images(true, "NVIDIA 550.54.14", 1));
    assert!(releases_undrawn_images(true, "", 0));
    // The window surfaces of the old Mesa implement the call.
    let (driver, packed) = mesa(25, 2, 8);
    assert!(releases_undrawn_images(false, &driver, packed));
}

/// A device that meets every requirement.
const CAPABLE: DeviceCaps = DeviceCaps {
    api_version: (1, 3),
    compute_queue: true,
    runtime_descriptor_array: true,
    sampled_image_array_non_uniform_indexing: true,
    descriptor_binding_partially_bound: true,
    descriptor_binding_sampled_image_update_after_bind: true,
    synchronization2: true,
    dynamic_rendering: true,
    max_update_after_bind_sampled_images: BITMAP_SLOTS,
    max_update_after_bind_resources: COMPOSE_SET_DESCRIPTORS,
    max_work_group_invocations: 256,
    max_work_group_size: [16, 16],
};

fn device(index: usize, name: &str, kind: DeviceKind, caps: DeviceCaps) -> DeviceInfo {
    DeviceInfo {
        index,
        name: name.to_owned(),
        kind,
        driver: String::new(),
        lacks: lacks(&caps),
    }
}

fn old() -> DeviceCaps {
    DeviceCaps {
        api_version: (1, 1),
        ..CAPABLE
    }
}

/// A CPU device, an integrated GPU and a discrete GPU, in the least preferred order.
fn three() -> Vec<DeviceInfo> {
    vec![
        device(0, "llvmpipe (LLVM 22)", DeviceKind::Cpu, CAPABLE),
        device(
            1,
            "Integrated Graphics (RADV)",
            DeviceKind::Integrated,
            CAPABLE,
        ),
        device(2, "Discrete Graphics 9000", DeviceKind::Discrete, CAPABLE),
    ]
}

#[test]
fn capable_device_lacks_nothing() {
    assert_eq!(lacks(&CAPABLE), Vec::<String>::new());
    let newer = DeviceCaps {
        api_version: (1, 4),
        max_update_after_bind_sampled_images: u32::MAX,
        max_update_after_bind_resources: u32::MAX,
        max_work_group_invocations: 1024,
        max_work_group_size: [1024, 1024],
        ..CAPABLE
    };
    assert_eq!(lacks(&newer), Vec::<String>::new());
    // A later major version also counts as at least 1.3.
    let next = DeviceCaps {
        api_version: (2, 0),
        ..CAPABLE
    };
    assert_eq!(lacks(&next), Vec::<String>::new());
}

#[test]
fn each_missing_requirement_is_named() {
    let cases: [(DeviceCaps, &str); 11] = [
        (old(), "Vulkan 1.3 (supports 1.1)"),
        (
            DeviceCaps {
                compute_queue: false,
                ..CAPABLE
            },
            "a compute queue",
        ),
        (
            DeviceCaps {
                runtime_descriptor_array: false,
                ..CAPABLE
            },
            "feature runtimeDescriptorArray",
        ),
        (
            DeviceCaps {
                sampled_image_array_non_uniform_indexing: false,
                ..CAPABLE
            },
            "feature shaderSampledImageArrayNonUniformIndexing",
        ),
        (
            DeviceCaps {
                descriptor_binding_partially_bound: false,
                ..CAPABLE
            },
            "feature descriptorBindingPartiallyBound",
        ),
        (
            DeviceCaps {
                descriptor_binding_sampled_image_update_after_bind: false,
                ..CAPABLE
            },
            "feature descriptorBindingSampledImageUpdateAfterBind",
        ),
        (
            DeviceCaps {
                synchronization2: false,
                ..CAPABLE
            },
            "feature synchronization2",
        ),
        (
            DeviceCaps {
                dynamic_rendering: false,
                ..CAPABLE
            },
            "feature dynamicRendering",
        ),
        (
            DeviceCaps {
                max_update_after_bind_sampled_images: 65535,
                ..CAPABLE
            },
            "65536 update-after-bind sampled images (supports 65535)",
        ),
        (
            DeviceCaps {
                max_work_group_invocations: 128,
                ..CAPABLE
            },
            "16x16 compute workgroups (supports 16x16, 128 invocations)",
        ),
        (
            DeviceCaps {
                max_work_group_size: [128, 8],
                ..CAPABLE
            },
            "16x16 compute workgroups (supports 128x8, 256 invocations)",
        ),
    ];
    for (caps, expected) in cases {
        assert_eq!(lacks(&caps), vec![expected.to_owned()]);
    }
}

/// Scenario "Exactly the bitmap slots" of "Descriptor limits of the whole set": the frame
/// target and the storage buffers of the set count too, and only the whole set is enough.
#[test]
fn descriptor_check_counts_the_whole_set() {
    let short = |resources| DeviceCaps {
        max_update_after_bind_resources: resources,
        ..CAPABLE
    };
    assert_eq!(COMPOSE_SET_DESCRIPTORS, 65542);
    for resources in [BITMAP_SLOTS, COMPOSE_SET_DESCRIPTORS - 1] {
        assert_eq!(
            lacks(&short(resources)),
            [format!(
                "65542 update-after-bind resources per stage, \
                 maxPerStageUpdateAfterBindResources (supports {resources})"
            )]
        );
        let info = device(0, "short", DeviceKind::Integrated, short(resources));
        assert!(!info.is_suitable());
        assert!(
            info.to_string()
                .contains("maxPerStageUpdateAfterBindResources"),
            "{info}"
        );
    }
    assert_eq!(lacks(&short(COMPOSE_SET_DESCRIPTORS)), Vec::<String>::new());
    // The bitmap slots alone are still held against the limits of sampled images.
    let few_images = DeviceCaps {
        max_update_after_bind_sampled_images: BITMAP_SLOTS - 1,
        ..CAPABLE
    };
    assert_eq!(lacks(&few_images).len(), 1);
}

#[test]
fn several_missing_requirements_are_all_reported() {
    let caps = DeviceCaps {
        compute_queue: false,
        runtime_descriptor_array: false,
        max_update_after_bind_sampled_images: 1024,
        ..CAPABLE
    };
    assert_eq!(lacks(&caps).len(), 3);
}

#[test]
fn auto_prefers_discrete_then_integrated_then_other() {
    let devices = three();
    assert_eq!(select(&DeviceChoice::Auto, &devices), Ok(2));
    assert_eq!(select(&DeviceChoice::Auto, &devices[..2]), Ok(1));
    assert_eq!(select(&DeviceChoice::Auto, &devices[..1]), Ok(0));
    let others = vec![
        device(0, "virtual", DeviceKind::Virtual, CAPABLE),
        device(1, "cpu", DeviceKind::Cpu, CAPABLE),
        device(2, "other", DeviceKind::Other, CAPABLE),
    ];
    assert_eq!(
        select(&DeviceChoice::Auto, &others),
        Ok(0),
        "first of equals"
    );
}

#[test]
fn auto_skips_unsuitable_devices_whatever_their_kind() {
    let devices = vec![
        device(0, "old discrete", DeviceKind::Discrete, old()),
        device(1, "old integrated", DeviceKind::Integrated, old()),
        device(2, "llvmpipe", DeviceKind::Cpu, CAPABLE),
    ];
    assert_eq!(select(&DeviceChoice::Auto, &devices), Ok(2));
}

#[test]
fn auto_takes_the_first_of_two_devices_of_the_preferred_kind() {
    let devices = vec![
        device(0, "integrated", DeviceKind::Integrated, CAPABLE),
        device(1, "discrete a", DeviceKind::Discrete, CAPABLE),
        device(2, "discrete b", DeviceKind::Discrete, CAPABLE),
    ];
    assert_eq!(select(&DeviceChoice::Auto, &devices), Ok(1));
}

#[test]
fn explicit_index_wins_over_the_preference_order() {
    let devices = three();
    for index in 0..3 {
        assert_eq!(select(&DeviceChoice::Index(index), &devices), Ok(index));
    }
}

#[test]
fn explicit_name_wins_over_the_preference_order() {
    let devices = three();
    let name = |text: &str| DeviceChoice::Name(text.to_owned());
    assert_eq!(select(&name("llvmpipe"), &devices), Ok(0));
    assert_eq!(select(&name("RADV"), &devices), Ok(1));
    assert_eq!(select(&name("radv"), &devices), Ok(1), "case is ignored");
    assert_eq!(select(&name("9000"), &devices), Ok(2));
    // Both GPUs match; the first one in enumeration order is taken.
    assert_eq!(select(&name("Graphics"), &devices), Ok(1));
}

#[test]
fn name_matching_several_devices_takes_the_first_suitable_one() {
    let devices = vec![
        device(0, "Radeon old", DeviceKind::Discrete, old()),
        device(1, "Radeon new", DeviceKind::Integrated, CAPABLE),
    ];
    assert_eq!(
        select(&DeviceChoice::Name("radeon".to_owned()), &devices),
        Ok(1)
    );
}

#[test]
fn missing_explicit_device_fails_without_falling_back() {
    let devices = three();
    for choice in [
        DeviceChoice::Index(3),
        DeviceChoice::Name("GeForce".to_owned()),
    ] {
        let error = select(&choice, &devices).unwrap_err();
        assert_eq!(
            error,
            DeviceError::NotFound {
                choice: choice.to_string(),
                devices: devices.clone(),
            }
        );
        let text = error.to_string();
        for device in &devices {
            assert!(text.contains(&device.name), "{text}");
        }
    }
}

#[test]
fn unsuitable_explicit_device_fails_without_falling_back() {
    let devices = vec![
        device(0, "old discrete", DeviceKind::Discrete, old()),
        device(1, "llvmpipe", DeviceKind::Cpu, CAPABLE),
    ];
    for choice in [DeviceChoice::Index(0), DeviceChoice::Name("old".to_owned())] {
        let error = select(&choice, &devices).unwrap_err();
        assert_eq!(
            error,
            DeviceError::Unsuitable {
                choice: choice.to_string(),
                devices: devices.clone(),
            }
        );
        let text = error.to_string();
        assert!(text.contains("0: old discrete (discrete GPU): lacks Vulkan 1.3"));
        assert!(text.contains("1: llvmpipe (CPU): suitable"), "{text}");
    }
}

#[test]
fn no_suitable_device_error_names_every_device_and_what_it_lacks() {
    let weak = DeviceCaps {
        runtime_descriptor_array: false,
        descriptor_binding_partially_bound: false,
        ..CAPABLE
    };
    let devices = vec![
        device(0, "Old Card", DeviceKind::Discrete, old()),
        device(1, "Weak Chip", DeviceKind::Integrated, weak),
    ];
    let error = select(&DeviceChoice::Auto, &devices).unwrap_err();
    assert_eq!(
        error,
        DeviceError::NoSuitableDevice {
            devices: devices.clone(),
        }
    );
    assert_eq!(
        error.to_string(),
        "no Vulkan device meets the requirements. Available devices:\n  \
         0: Old Card (discrete GPU): lacks Vulkan 1.3 (supports 1.1)\n  \
         1: Weak Chip (integrated GPU): lacks feature runtimeDescriptorArray; \
         feature descriptorBindingPartiallyBound"
    );
}

#[test]
fn no_device_at_all_is_reported_as_such() {
    let error = select(&DeviceChoice::Auto, &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "no Vulkan device meets the requirements. No Vulkan device is available."
    );
    assert!(matches!(
        select(&DeviceChoice::Index(0), &[]),
        Err(DeviceError::NotFound { .. })
    ));
}

#[test]
fn presenting_family_is_the_first_that_computes_draws_and_presents() {
    let device = device(1, "Chip", DeviceKind::Integrated, CAPABLE);
    let family = |compute, graphics, present| FamilyCaps {
        compute,
        graphics,
        present,
    };
    let families = [
        family(true, false, false),
        family(true, true, false),
        family(false, true, true),
        family(true, true, true),
        family(true, true, true),
    ];
    assert_eq!(presenting_family(&device, true, &families), Ok(3));
}

#[test]
fn device_that_cannot_present_is_refused_by_name() {
    let device = device(2, "Compute Brick 3000", DeviceKind::Discrete, CAPABLE);
    let family = |compute, graphics, present| FamilyCaps {
        compute,
        graphics,
        present,
    };

    let error = presenting_family(
        &device,
        true,
        &[family(true, true, false), family(true, false, true)],
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Vulkan device `Compute Brick 3000` cannot present to the surface: none of its \
         queue families can compute, draw and present to that surface"
    );

    let error = presenting_family(&device, false, &[family(true, true, true)]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Vulkan device `Compute Brick 3000` cannot present to the surface: it lacks the \
         VK_KHR_swapchain extension"
    );
    assert!(matches!(error, DeviceError::CannotPresent { .. }));
}

#[test]
fn pacing_extensions_need_their_dependencies_and_features() {
    let all = PacingCaps {
        surface_capabilities2: true,
        present_timing_extension: true,
        present_id2_extension: true,
        present_wait2_extension: true,
        calibrated_timestamps_extension: true,
        present_timing: true,
        present_at_absolute_time: true,
        present_id2: true,
        present_wait2: true,
    };
    let both = PacingSupport {
        timing: true,
        wait: true,
    };
    assert_eq!(pacing_support(&all), both);
    // What the CPU device of the tests offers.
    assert_eq!(
        pacing_support(&PacingCaps::default()),
        PacingSupport::default()
    );
    let neither: [fn(&mut PacingCaps); 6] = [
        |caps| caps.surface_capabilities2 = false,
        |caps| caps.present_timing_extension = false,
        |caps| caps.present_id2_extension = false,
        |caps| caps.calibrated_timestamps_extension = false,
        |caps| caps.present_timing = false,
        |caps| caps.present_at_absolute_time = false,
    ];
    for remove in neither {
        let mut caps = all;
        remove(&mut caps);
        assert_eq!(pacing_support(&caps), PacingSupport::default());
    }
    let timing_only: [fn(&mut PacingCaps); 3] = [
        |caps| caps.present_wait2_extension = false,
        |caps| caps.present_id2 = false,
        |caps| caps.present_wait2 = false,
    ];
    for remove in timing_only {
        let mut caps = all;
        remove(&mut caps);
        let expected = PacingSupport {
            timing: true,
            wait: false,
        };
        assert_eq!(pacing_support(&caps), expected);
    }
}

#[test]
fn surface_candidates_follow_the_preference_order_or_the_explicit_choice() {
    let devices = three();
    assert_eq!(
        surface_candidates(&DeviceChoice::Auto, &devices),
        Ok(vec![2, 1, 0])
    );
    let mixed = vec![
        device(0, "old discrete", DeviceKind::Discrete, old()),
        device(1, "llvmpipe", DeviceKind::Cpu, CAPABLE),
        device(2, "integrated", DeviceKind::Integrated, CAPABLE),
        device(3, "virtual", DeviceKind::Virtual, CAPABLE),
    ];
    assert_eq!(
        surface_candidates(&DeviceChoice::Auto, &mixed),
        Ok(vec![2, 1, 3])
    );
    // An explicit choice is one device and never widens to another.
    assert_eq!(
        surface_candidates(&DeviceChoice::Name("llvmpipe".to_owned()), &devices),
        Ok(vec![0])
    );
    assert_eq!(
        surface_candidates(&DeviceChoice::Index(1), &devices),
        Ok(vec![1])
    );
    assert!(matches!(
        surface_candidates(&DeviceChoice::Index(0), &mixed),
        Err(DeviceError::Unsuitable { .. })
    ));
    assert!(matches!(
        surface_candidates(&DeviceChoice::Auto, &mixed[..1]),
        Err(DeviceError::NoSuitableDevice { .. })
    ));
}

#[test]
fn device_description_includes_the_driver_when_known() {
    let mut info = device(1, "Chip", DeviceKind::Integrated, CAPABLE);
    info.driver = "radv Mesa 26".to_owned();
    assert_eq!(
        info.to_string(),
        "1: Chip (integrated GPU, radv Mesa 26): suitable"
    );
}
