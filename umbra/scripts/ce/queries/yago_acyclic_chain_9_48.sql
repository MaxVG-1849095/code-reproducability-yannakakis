select count(*) from yago5 yago5_0, yago21, yago5 yago5_2, yago5 yago5_3, yago5 yago5_4, yago4, yago58, yago23 yago23_7, yago23 yago23_8 where yago5_0.s = yago21.s and yago21.d = yago5_2.d and yago5_2.s = yago5_3.s and yago5_3.d = yago5_4.d and yago5_4.s = yago4.d and yago4.s = yago58.d and yago58.s = yago23_7.s and yago23_7.d = yago23_8.d;