#[cfg(test)]
mod unit_tests {
    use crate::scheduler::from_scheduler::Response;
    use crate::scheduler::response_correct::CardToView::NoCard;
    use crate::{proc_cards, Card, SplitTestManager, SplitTestConfig, AlgorithmParams, Variant};
    use std::collections::HashSet;
    use std::time::Duration;
    use chrono::{Utc, TimeZone};

    #[test]
    fn no_cards_no_nothing() {
        let h = HashSet::new();
        let ret = proc_cards(vec![], h, "{}".to_string(), 0, true);
        let ret = match ret {
            Response::GoodValue(g) => g,
            Response::BadValue(e) => panic!("wrong type: {:?}", e),
        };
        assert_eq!(ret.card_to_view.unwrap(), NoCard(Default::default()));
    }

    #[test]
    fn card_grading_progression() {
        let mut m = Card::new();

        assert_eq!(m.get_due_timestamp(), None);
        assert_eq!(m.get_interval(), None);
        assert_eq!(m.last_seen(), None);

        let orig = 60 * 60 * 24 * 365;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 15)));
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15 * 4));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 15 * 4)));
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15 * 4 * 4));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 15 * 4 * 4)));
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 15 * 4 * 4 * 4));
        assert_eq!(
            m.get_interval(),
            Some(Duration::from_secs(60 * 15 * 4 * 4 * 4))
        );
        assert_eq!(m.last_seen(), Some(orig));

        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);

        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 60 * 24));
        assert_eq!(m.get_interval(), Some(Duration::from_secs(60 * 60 * 24)));
        assert_eq!(m.last_seen(), Some(orig));
        
        let orig = m.get_due_timestamp().unwrap() + 1;
        m.grade(orig, true);
        assert_eq!(
            m.get_interval(),
            Some(Duration::from_secs(60 * 60 * 24 * 2))
        );
        assert_eq!(m.last_seen(), Some(orig));
        assert_eq!(m.get_due_timestamp(), Some(orig + 60 * 60 * 24 * 2));
    }

    #[test]
    fn failed_single_card_prod() {
        let mut m = Card::new();

        assert_eq!(m.get_due_timestamp(), None);
        assert_eq!(m.get_interval(), None);
        assert_eq!(m.last_seen(), None);
        
        // Test case from production data
        m.grade(0, false);
        m.grade(14, false);
        assert_eq!(29, m.get_due_timestamp().unwrap());
        m.grade(26, true);
        assert_eq!(926, m.get_due_timestamp().unwrap());
        m.grade(29, true);
        assert_eq!(1829, m.get_due_timestamp().unwrap());
    }

    #[test]
    fn card_short_term_progression() {
        let mut card = Card::new();
        
        // First correct answer - should go to short term
        card.grade(1000, true);
        assert!(card.is_short_term());
        assert_eq!(card.get_pos(), Some(0));
        
        // Second correct answer - should progress
        let due_time = card.get_due_timestamp().unwrap();
        card.grade(due_time + 1, true);
        assert!(card.is_short_term());
        assert_eq!(card.get_pos(), Some(1));
    }

    #[test]
    fn card_wrong_answer_handling() {
        let mut card = Card::new();
        
        // Wrong answer should set to wrong state
        card.grade(1000, false);
        assert!(card.is_short_term());
        assert_eq!(card.get_pos(), Some(0));
        assert_eq!(card.get_interval(), Some(Duration::from_secs(15)));
    }

    #[test]
    fn card_long_term_transition() {
        let mut card = Card::new();
        
        // Progress through short term stages
        card.grade(1000, true);
        let mut due_time = card.get_due_timestamp().unwrap();
        
        // Continue grading correctly until we reach long term
        for _ in 0..10 {
            card.grade(due_time + 1, true);
            due_time = card.get_due_timestamp().unwrap();
            
            // Check if we've transitioned to long term
            if !card.is_short_term() {
                break;
            }
        }
        
        // Should eventually reach long term
        assert!(!card.is_short_term());
    }

    // Split Testing Unit Tests
    #[test]
    fn split_test_manager_creation() {
        let manager = SplitTestManager::new();
        assert!(manager.configs.is_empty());
        assert!(manager.user_assignments.is_empty());
    }

    #[test]
    fn split_test_experiment_assignment() {
        let mut manager = SplitTestManager::new();
        
        let config = SplitTestConfig {
            experiment_id: "test_experiment".to_string(),
            variant_a_percentage: 50,
            variant_a_params: AlgorithmParams::default(),
            variant_b_params: AlgorithmParams {
                short_term_multiplier: Some(3.0),
                ..Default::default()
            },
            start_date: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            end_date: Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap(),
            is_active: true,
        };
        
        manager.add_experiment(config);
        
        // Test consistent assignment
        let variant1 = manager.assign_user_to_variant("user1", "test_experiment");
        let variant2 = manager.assign_user_to_variant("user1", "test_experiment");
        
        assert!(variant1.is_some());
        assert_eq!(variant1, variant2);
    }

    #[test]
    fn split_test_algorithm_params() {
        let mut manager = SplitTestManager::new();
        
        let variant_a_params = AlgorithmParams {
            short_term_multiplier: Some(4.0),
            ..Default::default()
        };
        
        let variant_b_params = AlgorithmParams {
            short_term_multiplier: Some(3.0),
            ..Default::default()
        };
        
        let config = SplitTestConfig {
            experiment_id: "param_test".to_string(),
            variant_a_percentage: 50,
            variant_a_params,
            variant_b_params,
            start_date: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            end_date: Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap(),
            is_active: true,
        };
        
        manager.add_experiment(config);
        
        // Assign user and get params
        let variant = manager.assign_user_to_variant("user1", "param_test");
        assert!(variant.is_some());
        
        let params = manager.get_algorithm_params("user1", "param_test");
        assert!(params.is_some());
        
        let params = params.unwrap();
        match variant.unwrap() {
            Variant::A => assert_eq!(params.short_term_multiplier, Some(4.0)),
            Variant::B => assert_eq!(params.short_term_multiplier, Some(3.0)),
        }
    }

    #[test]
    fn split_test_inactive_experiment() {
        let mut manager = SplitTestManager::new();
        
        let config = SplitTestConfig {
            experiment_id: "inactive_test".to_string(),
            variant_a_percentage: 50,
            variant_a_params: AlgorithmParams::default(),
            variant_b_params: AlgorithmParams::default(),
            start_date: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            end_date: Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap(),
            is_active: false, // Inactive experiment
        };
        
        manager.add_experiment(config);
        
        let variant = manager.assign_user_to_variant("user1", "inactive_test");
        assert!(variant.is_none());
    }

    #[test]
    fn split_test_expired_experiment() {
        let mut manager = SplitTestManager::new();
        
        let config = SplitTestConfig {
            experiment_id: "expired_test".to_string(),
            variant_a_percentage: 50,
            variant_a_params: AlgorithmParams::default(),
            variant_b_params: AlgorithmParams::default(),
            start_date: Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            end_date: Utc.with_ymd_and_hms(2020, 12, 31, 23, 59, 59).unwrap(), // Expired
            is_active: true,
        };
        
        manager.add_experiment(config);
        
        let variant = manager.assign_user_to_variant("user1", "expired_test");
        assert!(variant.is_none());
    }

    #[test]
    fn algorithm_params_default_values() {
        let params = AlgorithmParams::default();
        
        assert_eq!(params.short_term_multiplier, Some(4.0));
        assert_eq!(params.long_term_base_multiplier, Some(2.0));
        assert_eq!(params.forgetting_curve_factor, Some(0.9));
        assert_eq!(params.max_interval_days, Some(365));
        assert_eq!(params.wrong_answer_interval, Some(15));
        assert_eq!(params.initial_interval, Some(15 * 60));
    }
}
