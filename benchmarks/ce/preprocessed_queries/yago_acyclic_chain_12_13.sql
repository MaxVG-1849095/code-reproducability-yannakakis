select count(*) from yago3, yago5, yago39_2, yago62, yago1, yago21_5, yago13, yago39_7, yago36_8, yago21_9, yago22, yago36_11 where yago3.d = yago1.d and yago5.s = yago39_2.s and yago5.d = yago62.s and yago39_2.d = yago36_11.d and yago1.s = yago21_5.s and yago21_5.d = yago13.d and yago13.s = yago39_7.s and yago39_7.d = yago36_8.d and yago36_8.s = yago21_9.s and yago21_9.d = yago22.d and yago22.s = yago36_11.s;