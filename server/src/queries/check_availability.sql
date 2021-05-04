SELECT EXISTS(SELECT "id" FROM "recordings" WHERE "name" = $1);
