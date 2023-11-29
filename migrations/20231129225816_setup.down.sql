-- i think these automatically are dropped when we drop the tables, but doesn't
-- hurt to do it here too
DROP TRIGGER execution_notify ON execution;
DROP TRIGGER flag_notify ON flag;
DROP TRIGGER exploit_notify ON exploit;

DROP TABLE exploit, execution, flag, service, target, team;

DROP FUNCTION notify_trigger();
