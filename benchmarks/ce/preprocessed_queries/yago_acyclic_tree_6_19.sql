select count(*) from yago5_0, yago13, yago5_2, yago22, yago54, yago21 where yago5_0.s = yago13.s and yago13.s = yago5_2.s and yago13.d = yago22.d and yago5_2.d = yago21.d and yago22.s = yago54.s;