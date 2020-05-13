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

const {toObjectId} = require("../common/util")

const express = require('express');
const router = express.Router();
const CountsService = require("../db/services/CountService")

router.get('/:sim_id/interventions', (req, res) => {
  const simulationId = toObjectId(req.params.sim_id)
  CountsService.fetchInterventionsForSimulation(simulationId)
    .then(interventions => {
      res.send(interventions)
    })
})

module.exports = router;