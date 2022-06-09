/*
 * EpiRust
 * Copyright (c) 2020  ThoughtWorks, Inc.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 */


use crate::agent;
use crate::geography::{Area, Grid};
use crate::geography::Point;
use crate::random_wrapper::RandomWrapper;
use crate::agent::Citizen;
use crate::disease_state_machine::State;
use crate::listeners::events::counts::Counts;
use crate::travel_plan::Migrator;
use std::collections::hash_map::{IterMut, Iter};
use fnv::FnvHashMap;
use crate::commute::Commuter;
use crate::custom_types::{CoOrdinate, Count, Size};

#[derive(Clone)]
pub struct AgentLocationMap {
    grid_size: Size,
    agent_cell: FnvHashMap<Point, agent::Citizen>,
}

impl AgentLocationMap {
    pub fn init_with_capacity(&mut self, size: usize) {
        self.agent_cell = FnvHashMap::with_capacity_and_hasher(size, Default::default());
    }

    pub fn new(grid_size: Size, agent_list: &[agent::Citizen], points: &[Point]) -> AgentLocationMap {
        debug!("{} agents and {} starting points", agent_list.len(), points.len());
        let mut map: FnvHashMap<Point, agent::Citizen> = FnvHashMap::with_capacity_and_hasher(agent_list.len(), Default::default());
        for i in 0..agent_list.len() {
            map.insert(points[i], agent_list[i].clone());
        }

        AgentLocationMap { grid_size, agent_cell: map }
    }

    pub fn move_agent(&self, old_cell: Point, new_cell: Point) -> Point {
        if self.is_cell_vacant(&new_cell) {
            return new_cell;
        }
        old_cell
    }

    pub fn goto_hospital(&self, hospital_area: &Area, cell: Point, citizen: &mut agent::Citizen) -> (bool, Point) {
        let vacant_hospital_cell = hospital_area.iter().find(|cell| {
            self.is_cell_vacant(cell)
        });
        match vacant_hospital_cell {
            Some(x) => (true, self.move_agent(cell, x)),
            None => {
                (false,
                 self.move_agent(cell, citizen.home_location.get_random_point(&mut RandomWrapper::new())))
            }
        }
    }

//    pub fn print(&self){
//        for (k,v) in self.agent_cell.iter(){
//            println!("x:{}, y:{} - id:{} infected:{} working:{} Transport:{}", k.x, k.y, v.id, v.is_infected(), v.working, v.uses_public_transport);
//        }
//    }

    pub fn get_agent_for(&self, cell: &Point) -> Option<&agent::Citizen> {
        self.agent_cell.get(cell)
    }

    pub fn is_point_in_grid(&self, point: &Point) -> bool {
        let end_coordinate_of_grid = self.grid_size as CoOrdinate;
        point.x >= 0 && point.y >= 0 && point.x < end_coordinate_of_grid && point.y < end_coordinate_of_grid
    }

    pub fn is_cell_vacant(&self, cell: &Point) -> bool {
        !self.agent_cell.contains_key(cell)
    }

    pub fn remove_migrators(&mut self, outgoing: &Vec<(Point, Migrator)>, counts: &mut Counts, grid: &mut Grid) {
        if outgoing.is_empty() {
            return;
        }
        debug!("Removing {} outgoing travellers", outgoing.len());
        for (point, migrator) in outgoing {
            match migrator.state_machine.state {
                State::Susceptible { .. } => { counts.remove_susceptible(1) },
                State::Exposed { .. } => { counts.remove_exposed(1) }
                State::Infected { .. } => { counts.remove_infected(1) },
                State::Recovered { .. } => { counts.remove_recovered(1) },
                State::Deceased { .. } => { panic!("Deceased agent should not travel!") },
            }
            match self.agent_cell.remove(point) {
                None => {
                    panic!("Trying to remove citizen {:?} from location {:?}, but no citizen is present at this location!",
                           migrator.id, point)
                }
                Some(citizen) => {
                    grid.remove_house_occupant(&citizen.home_location);
                    if citizen.is_working() {
                        grid.remove_office_occupant(&citizen.work_location);
                    }
                }
            }
        }
    }

    pub fn assimilate_migrators(&mut self, incoming: &mut Vec<Migrator>, grid: &mut Grid, counts: &mut Counts,
                                rng: &mut RandomWrapper) {
        if incoming.is_empty() {
            return;
        }
        debug!("Assimilating {} incoming migrators", incoming.len());
        let mut new_citizens: Vec<Citizen> = Vec::with_capacity(incoming.len());
        for migrator in incoming {
            let house = grid.choose_house_with_free_space(rng);
            let office = if migrator.working {
                grid.choose_office_with_free_space(rng)
            } else {
                house.clone()
            };
            let transport_location = house.get_random_point(rng); // Fixme
            new_citizens.push(Citizen::from_migrator(migrator, house.clone(), office.clone(), transport_location, grid.housing_area.clone()));
            grid.add_house_occupant(&house.clone());
            if migrator.working {
                grid.add_office_occupant(&office.clone())
            }
        }
        for c in new_citizens {
            match c.state_machine.state {
                State::Susceptible { .. } => { counts.update_susceptible(1) }
                State::Exposed { .. } => { counts.update_exposed(1) }
                State::Infected { .. } => { counts.update_infected(1) }
                State::Recovered { .. } => { counts.update_recovered(1) }
                State::Deceased { .. } => { panic!("Should not receive deceased agent!") }
            }
            let p = self.random_starting_point(&grid.housing_area, rng);
            let result = self.agent_cell.insert(p, c);
            assert!(result.is_none());
        }
    }

    pub fn assimilate_commuters(&mut self, incoming: &mut Vec<Commuter>, grid: &mut Grid, counts: &mut Counts,
                                rng: &mut RandomWrapper) {
        if incoming.is_empty() {
            return;
        }
        debug!("Assimilating {} incoming commuters", incoming.len());
        let mut new_citizens: Vec<Citizen> = Vec::with_capacity(incoming.len());
        for commuter in incoming {
            let house = grid.choose_house_with_free_space(rng);
            let office = grid.choose_office_with_free_space(rng);
            let transport_location = house.get_random_point(rng);
            new_citizens.push(Citizen::from_commuter(commuter,  transport_location, grid.housing_area.clone()));
            grid.add_office_occupant(&office.clone())
        }
        for c in new_citizens {
            match c.state_machine.state {
                State::Susceptible { .. } => { counts.update_susceptible(1) }
                State::Exposed { .. } => { counts.update_exposed(1) }
                State::Infected { .. } => { counts.update_infected(1) }
                State::Recovered { .. } => { counts.update_recovered(1) }
                State::Deceased { .. } => { panic!("Should not receive deceased agent!") }
            }
        }
    }

    fn random_starting_point(&self, area: &Area, rng: &mut RandomWrapper) -> Point {
        loop {
            let point = area.get_random_point(rng);
            if !self.agent_cell.contains_key(&point) {
                return point
            }
        }
    }

    pub fn current_population(&self) -> Count {
        self.agent_cell.len() as Count
    }

    pub fn iter(&self) -> Iter<'_, Point, Citizen> {
        self.agent_cell.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, Point, Citizen> {
        self.agent_cell.iter_mut()
    }

    pub fn clear(&mut self) {
        self.agent_cell.clear();
    }

    pub fn get(&self, point: &Point) -> Option<&Citizen> {
        self.agent_cell.get(point)
    }

    pub fn insert(&mut self, point: Point, citizen: Citizen) -> Option<Citizen> {
        self.agent_cell.insert(point, citizen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random_wrapper::RandomWrapper;
    use crate::agent::WorkStatus;

    fn before_each() -> AgentLocationMap {
        let mut rng = RandomWrapper::new();
        let points = vec![Point { x: 0, y: 1 }, Point { x: 1, y: 0 }];
        let engine_id = "engine1".to_string();
        let home_locations = vec![Area::new(engine_id.clone(),Point::new(0, 0), Point::new(2, 2)), Area::new(engine_id.clone(),Point::new(3, 0), Point::new(4, 2))];

        let work_locations = vec![Area::new(engine_id.clone(),Point::new(5, 0), Point::new(6, 2)), Area::new(engine_id.clone(),Point::new(7, 0), Point::new(8, 2))];
        let work_status = WorkStatus::NA {};

        let agents = vec![agent::Citizen::new(home_locations[0].clone(), work_locations[0].clone(), points[0], false, false, work_status, &mut rng),
                          agent::Citizen::new(home_locations[1].clone(), work_locations[0].clone(), points[0], true, true, work_status, &mut rng)];
        AgentLocationMap::new(5, &agents, &points)
    }

    #[test]
    fn new() {
        let map = before_each();

        assert_eq!(map.grid_size, 5);
    }

    #[test]
    fn should_goto_hospital() {
        let mut rng = RandomWrapper::new();
        let points = vec![Point { x: 0, y: 1 }, Point { x: 1, y: 0 }];
        let work_status = WorkStatus::Normal {};
        let engine_id = "engine1".to_string();
        let home_locations = vec![Area::new(engine_id.clone(),Point::new(0, 0), Point::new(2, 2)), Area::new(engine_id.clone(),Point::new(3, 0), Point::new(4, 2))];

        let work_locations = vec![Area::new(engine_id.clone(),Point::new(5, 0), Point::new(6, 2)), Area::new(engine_id.clone(),Point::new(7, 0), Point::new(8, 2))];
        let citizen1 = agent::Citizen::new(home_locations[0].clone(), work_locations[1].clone(), points[0], false, false, work_status, &mut rng);
        let citizen2 = agent::Citizen::new(home_locations[1].clone(), work_locations[0].clone(), points[0], true, true, work_status, &mut rng);
        let agents = vec![citizen1.clone(), citizen2.clone()];
        let map = AgentLocationMap::new(5, &agents, &points);
        let hospital = Area::new(engine_id.clone(),Point::new(2, 2), Point::new(4, 4));
        let result = map.goto_hospital(&hospital, points[0], &mut citizen1.clone());

        assert_eq!(result.0, true);
        assert_eq!(result.1, Point::new(2, 2));
    }

    #[test]
    fn should_goto_home_location_when_hospital_full() {
        let mut rng = RandomWrapper::new();
        let engine_id = "engine1".to_string();
        let points = vec![Point::new(0, 0), Point::new(0, 1), Point::new(1, 0), Point::new(1, 1)];
        let home = Area::new(engine_id.clone(),Point::new(0, 0), Point::new(2, 2));
        let work_status = WorkStatus::NA {};

        let work = Area::new(engine_id.clone(),Point::new(5, 0), Point::new(6, 2));
        let citizen1 = agent::Citizen::new(home.clone(), work.clone(), points[0], false, false, work_status, &mut rng);
        let citizen2 = agent::Citizen::new(home.clone(), work.clone(), points[0], false, false, work_status, &mut rng);
        let citizen3 = agent::Citizen::new(home.clone(), work.clone(), points[0], false, false, work_status, &mut rng);
        let citizen4 = agent::Citizen::new(home.clone(), work.clone(), points[0], false, false, work_status, &mut rng);
        let agents = vec![citizen1.clone(), citizen2.clone(), citizen3.clone(), citizen4.clone()];
        let map = AgentLocationMap::new(5, &agents, &points);
        let hospital = Area::new(engine_id.clone(),Point::new(0, 0), Point::new(1, 1));

        let result = map.goto_hospital(&hospital, points[0], &mut citizen1.clone());

        assert_eq!(result.0, false);
        assert_eq!(citizen1.clone().home_location.contains(&result.1), true);
    }

    #[test]
    fn should_return_true_when_point_is_in_grid() {
        let map = before_each();
        let points = vec![Point::new(0, 0), Point::new(4, 4), Point::new(2, 2)];
        for point in points {
            assert!(map.is_point_in_grid(&point))
        }
    }

    #[test]
    fn should_return_false_when_point_is_out_of_grid() {
        let map = before_each();
        let points = vec![Point::new(-1, -1), Point::new(5, 5), Point::new(2, 12)];
        for point in points {
            assert!(!map.is_point_in_grid(&point))
        }
    }
}
