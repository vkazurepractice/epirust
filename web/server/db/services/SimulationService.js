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

const {Simulation, SimulationStatus} = require("../models/Simulation");

const markSimulationEnd = async (simulationId) => {
  const query = {simulation_id: simulationId};
  const update = {status: SimulationStatus.FINISHED};
  await Simulation.updateOne(query, update, {upsert: true})
};

const markGridConsumptionFinished = async (simulationId) => {
  const query = {simulation_id: simulationId};
  const update = {simulation_id: simulationId, grid_consumption_finished: true};
  const simulationUpdate = Simulation.updateOne(query, update, {upsert: true});
  await simulationUpdate.exec()
};

module.exports = {markSimulationEnd, markGridConsumptionFinished};