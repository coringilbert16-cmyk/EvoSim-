        let spectrum = crate::harmonics::ToneSpectrum {
            components: vec![crate::harmonics::ToneComponent {
                frequency_hz: 440.0,
                amplitude: 0.75,
                phase_radians: 0.25,
            }],
        };

        reinforce_memory_point(
            &mut organism,
            12.0,
            34.0,
            0.5,
            1,
            &spectrum,
            crate::decision::ActionConsequence {
                energy_delta: 1.0,
                stress_delta: 0.0,
                developmental_delta: 0.0,
            },
        );

        assert_eq!(organism.memory.len(), 1);
        assert_eq!(organism.memory[0].spectrum, spectrum);
        assert_eq!(
            organism.memory[0].consequence,
            Some(crate::decision::ActionConsequence {
                energy_delta: 1.0,
                stress_delta: 0.0,
                developmental_delta: 0.0,
            })
        );
    }

    #[test]
    fn perception_memory_is_level_one_until_an_outcome_occurs() {
        let mut organism = Simulation::create_initial_organism();
        let spectrum = crate::harmonics::ToneSpectrum::empty();

        Simulation::remember_perception(&mut organism, 12.0, 34.0, 0.5, 1, &spectrum);
        assert_eq!(organism.memory.len(), 1);
        assert_eq!(organism.memory[0].consequence, None);
        let perception_strength = organism.memory[0].strength;

        reinforce_memory_point(
            &mut organism,
            12.0,
            34.0,
            0.5,
            1,
            &spectrum,
            crate::decision::ActionConsequence {
                energy_delta: -1.0,
                stress_delta: 1.0,
                developmental_delta: 0.0,
            },
        );

        assert_eq!(
            organism.memory[0].consequence,
            Some(crate::decision::ActionConsequence {
                energy_delta: -1.0,
                stress_delta: 1.0,
                developmental_delta: 0.0,
            })
        );
        assert!(organism.memory[0].strength > perception_strength);
    }

    #[test]
    fn memory_capacity_uses_diminishing_area_returns() {
        let minimum = crate::cavity::GenomeCavity {