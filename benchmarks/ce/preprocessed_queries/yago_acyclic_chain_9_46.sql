select count(*) from yago36, yago58, yago22_2, yago5, yago22_4, yago57_5, yago57_6, yago13, yago23 where yago36.s = yago58.s and yago58.d = yago22_2.s and yago22_2.d = yago5.d and yago5.s = yago22_4.s and yago22_4.d = yago57_5.s and yago57_5.d = yago57_6.d and yago57_6.s = yago13.d and yago13.s = yago23.s;